#[cfg(feature = "leak-resistant")]
use crate::bacc::Bacc;
use crate::ref_panel::RefPanel;
use crate::{Input, Real};
use lazy_static::lazy_static;
use ndarray::{s, Array1, Array2, ArrayView1, Zip};
use std::convert::TryFrom;
#[cfg(feature = "leak-resistant")]
use timing_shield::{TpEq, TpOrd};

pub const BACKGROUND: f64 = 1e-5;

#[cfg(not(feature = "leak-resistant"))]
mod cons {
    pub const __NORM_THRESHOLD: f64 = 1e-20;
    pub const __NORM_SCALE_FACTOR: f64 = 1e10;
    pub const __E: f64 = 1e-30;
}

#[cfg(feature = "leak-resistant")]
mod cons {
    pub const __NORM_THRESHOLD: f64 = 1e-20;
    pub const __NORM_SCALE_FACTOR: f64 = 1e10;
    pub const __E: f64 = 1e-30;
}

lazy_static! {
    static ref _NORM_THRESHOLD: Real = cons::__NORM_THRESHOLD.into();
    static ref _NORM_SCALE_FACTOR: Real = cons::__NORM_SCALE_FACTOR.into();
    static ref _E: Real = cons::__E.into();
}

#[allow(non_snake_case)]
pub fn impute_chunk(
    _chunk_id: usize,
    thap: ArrayView1<Input>,
    ref_panel: &RefPanel,
) -> Array1<Real> {
    assert!(thap.len() == ref_panel.n_markers);

    // Put all constants on stack
    let NORM_THRESHOLD = *_NORM_THRESHOLD;
    let NORM_SCALE_FACTOR = *_NORM_SCALE_FACTOR;
    let E = *_E;

    let blocks = &ref_panel.blocks;
    let m = ref_panel.n_haps;
    let m_real: Real = u32::try_from(m).unwrap().into();

    let mut imputed = unsafe { Array1::<Real>::uninitialized(thap.len()) };

    let mut fwdcache = Vec::new();
    let mut fwdcache_norecom = Vec::new();
    let mut fwdcache_first = Vec::new();
    let mut fwdcache_all = unsafe { Array2::<Real>::uninitialized((blocks.len(), m)) };

    /* Forward pass */
    let mut sprob_all = Array1::<Real>::ones(m); // unnormalized
    let mut var_offset: usize = 0;

    // First position emission (edge case)
    #[cfg(not(feature = "leak-resistant"))]
    let cond = thap[0] != -1;

    #[cfg(feature = "leak-resistant")]
    // TODO: fix this leakage
    let cond = thap[0].expose() != -1;

    if cond {
        let block = &blocks[0];
        let err = 0.00999;
        let tsym = thap[0];

        #[cfg(not(feature = "leak-resistant"))]
        let afreq = if tsym == 1 {
            block.afreq[0]
        } else {
            1. - block.afreq[0]
        };

        Zip::from(&mut sprob_all)
            .and(&block.indmap)
            .apply(|p, &ind| {
                #[cfg(not(feature = "leak-resistant"))]
                let emi: Real = if tsym == block.rhap[[0, ind]] {
                    (1. - err) + err * afreq + BACKGROUND
                } else {
                    err * afreq + BACKGROUND
                }
                .into();

                #[cfg(feature = "leak-resistant")]
                let emi = Real::select_from_4_f64(
                    tsym.tp_eq(&1),
                    tsym.tp_eq(&block.rhap[[0, ind]]),
                    (1. - err) + err * block.afreq[0] + BACKGROUND,
                    err * block.afreq[0] + BACKGROUND,
                    (1. - err) + err * (1. - block.afreq[0]) + BACKGROUND,
                    err * (1. - block.afreq[0]) + BACKGROUND,
                );

                *p = emi;
            });
    }

    for b in 0..blocks.len() {
        let block = &blocks[b];

        fwdcache_all.slice_mut(s![b, ..]).assign(&sprob_all); // save cache

        let mut fwdprob = unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };
        let mut fwdprob_norecom =
            unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };

        // Fold probabilities

        #[cfg(not(feature = "leak-resistant"))]
        let mut sprob = {
            let mut sprob = Array1::<Real>::zeros(block.nuniq);
            for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
                sprob[ind] += p;
            }
            sprob
        };

        #[cfg(feature = "leak-resistant")]
        let mut sprob = {
            let sprob = (0..block.nuniq)
                .map(|i| {
                    block.rev_indmap[&i]
                        .iter()
                        .map(|&j| sprob_all[j])
                        .sum::<Real>()
                })
                .collect::<Vec<Real>>();
            Array1::from(sprob)
        };

        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        // Walk
        Zip::from(block.rprob.slice(s![..block.nvar - 1]))
            .and(thap.slice(s![var_offset + 1..var_offset + block.nvar]))
            .and(block.afreq.slice(s![1..]))
            .and(block.rhap.slice(s![1.., ..]).genrows())
            .and(fwdprob.slice_mut(s![1.., ..]).genrows_mut())
            .and(fwdprob_norecom.slice_mut(s![1.., ..]).genrows_mut())
            .apply(
                |&rec, &tsym, &block_afreq, rhap_row, mut fwdprob_row, mut fwdprob_norecom_row| {
                    // TODO: for some reason minimac ignores error prob in input m3vcf
                    //       and always uses 0.00999 as below. need to investigate further
                    //let err = block.eprob[j];
                    let err = 0.00999;

                    let rec_real: Real = rec.into();

                    // Transition
                    let mut sprob_tot = sprob.iter().sum::<Real>() * (rec_real / m_real);
                    sprob_norecom *= Real::from(1. - rec);
                    let mut complement: Real = (1. - rec).into();

                    // Lazy normalization (same as minimac)
                    #[cfg(not(feature = "leak-resistant"))]
                    if sprob_tot < NORM_THRESHOLD {
                        sprob_tot *= NORM_SCALE_FACTOR;
                        complement *= NORM_SCALE_FACTOR;
                        sprob_norecom *= NORM_SCALE_FACTOR;
                    }

                    #[cfg(feature = "leak-resistant")]
                    {
                        let scale = sprob_tot
                            .tp_lt(&NORM_THRESHOLD)
                            .select(NORM_SCALE_FACTOR, Real::ONE);
                        sprob_tot *= scale;
                        complement *= scale;
                        sprob_norecom *= scale;
                    }

                    sprob.assign(&(complement * &sprob + &block.clustsize * sprob_tot));

                    // Emission
                    // TODO: fix this leakage
                    #[cfg(not(feature = "leak-resistant"))]
                    let cond = tsym != -1;

                    #[cfg(feature = "leak-resistant")]
                    let cond = tsym.expose() != -1;

                    if cond {
                        #[cfg(not(feature = "leak-resistant"))]
                        let afreq = if tsym == 1 {
                            block_afreq
                        } else {
                            1. - block_afreq
                        };

                        Zip::from(&mut sprob)
                            .and(&mut sprob_norecom)
                            .and(&rhap_row)
                            .apply(|p, p_norecom, &rhap| {
                                #[cfg(not(feature = "leak-resistant"))]
                                let emi = if tsym == rhap {
                                    (1. - err) + err * afreq + BACKGROUND
                                } else {
                                    err * afreq + BACKGROUND
                                };

                                #[cfg(feature = "leak-resistant")]
                                let emi = Real::select_from_4_f64(
                                    tsym.tp_eq(&1),
                                    tsym.tp_eq(&rhap),
                                    (1. - err) + err * block_afreq + BACKGROUND,
                                    err * block_afreq + BACKGROUND,
                                    (1. - err) + err * (1. - block_afreq) + BACKGROUND,
                                    err * (1. - block_afreq) + BACKGROUND,
                                );

                                *p *= emi;
                                *p_norecom *= emi;
                            });
                    }

                    // Cache forward probabilities
                    fwdprob_row.assign(&sprob);
                    fwdprob_norecom_row.assign(&sprob_norecom);
                },
            );

        let sprob_recom = &sprob - &sprob_norecom;

        // Unfold probabilities
        if b < blocks.len() - 1 {
            // Skip last block
            Zip::from(&mut sprob_all)
                .and(&block.indmap)
                .apply(|p, &ui| {
                    // TODO: precompute ui terms outside of this for loop
                    *p = (sprob_recom[ui] / block.clustsize[ui])
                        + (*p * (sprob_norecom[ui] / (sprob_first[ui] + E)));
                });
        }

        fwdcache.push(fwdprob);
        fwdcache_norecom.push(fwdprob_norecom);
        fwdcache_first.push(sprob_first);

        var_offset += block.nvar - 1;
    }

    /* Backward pass */
    // TODO: refactor to remove redundancy with forward pass
    let mut sprob_all = Array1::<Real>::ones(m);
    let mut var_offset: usize = 0;
    for b in (0..blocks.len()).rev() {
        let block = &blocks[b];
        let fwdprob = &fwdcache[b];
        let fwdprob_norecom = &fwdcache_norecom[b];
        let fwdprob_first = &fwdcache_first[b];

        // Precompute joint fwd-bwd term for imputation;
        // same as "Constants" variable in minimac
        #[cfg(not(feature = "leak-resistant"))]
        let jprob = {
            let mut jprob = Array1::<Real>::zeros(block.nuniq);
            Zip::from(&block.indmap)
                .and(fwdcache_all.slice(s![b, ..]))
                .and(&sprob_all)
                .apply(|&ind, &c, &p| {
                    jprob[ind] += c * p;
                });
            jprob
        };

        #[cfg(feature = "leak-resistant")]
        let jprob = {
            let jprob = (0..block.nuniq)
                .map(|i| {
                    block.rev_indmap[&i]
                        .iter()
                        .map(|&j| fwdcache_all[[b, j]] * sprob_all[j])
                        .sum::<Real>()
                })
                .collect::<Vec<Real>>();
            Array1::from(jprob)
        };

        // Fold probabilities
        #[cfg(not(feature = "leak-resistant"))]
        let mut sprob = {
            let mut sprob = Array1::<Real>::zeros(block.nuniq);
            for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
                sprob[ind] += p;
            }
            sprob
        };

        #[cfg(feature = "leak-resistant")]
        let mut sprob = {
            let sprob = (0..block.nuniq)
                .map(|i| {
                    block.rev_indmap[&i]
                        .iter()
                        .map(|&j| sprob_all[j])
                        .sum::<Real>()
                })
                .collect::<Vec<Real>>();
            Array1::from(sprob)
        };

        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        // Walk
        for j in (1..block.nvar).rev() {
            let rec = block.rprob[j - 1];
            // TODO: for some reason minimac ignores error prob in input m3vcf
            //       and always uses 0.00999 as below. need to investigate further
            //let err = block.eprob[j];
            let err = 0.00999;
            let varind = thap.len() - (var_offset + (block.nvar - j));
            let tsym = thap[varind];

            // Impute
            let combined = {
                let x = &fwdprob_norecom.slice(s![j, ..]) * &sprob_norecom;
                &jprob * &(x.clone() / (fwdprob_first * &sprob_first + E))
                    + (&fwdprob.slice(s![j, ..]) * &sprob - x) / &block.clustsize
            };

            #[cfg(not(feature = "leak-resistant"))]
            let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![j, ..])).fold(
                (0., 0.),
                |mut acc, &c, &rsym| {
                    if rsym == 1 {
                        acc.1 += c;
                        acc
                    } else {
                        acc.0 += c;
                        acc
                    }
                },
            );

            #[cfg(feature = "leak-resistant")]
            let (p0, p1) = {
                let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![j, ..])).fold(
                    (Bacc::init(), Bacc::init()),
                    |mut acc, &c, &rsym| {
                        if rsym == 1 {
                            acc.1 += c;
                            acc
                        } else {
                            acc.0 += c;
                            acc
                        }
                    },
                );
                (p0.result(), p1.result())
            };

            imputed[varind] = p1 / (p1 + p0);

            // Emission
            // TODO: fix this leakage
            #[cfg(not(feature = "leak-resistant"))]
            let cond = tsym != -1;

            #[cfg(feature = "leak-resistant")]
            let cond = tsym.expose() != -1;

            if cond {
                #[cfg(not(feature = "leak-resistant"))]
                let afreq = if tsym == 1 {
                    block.afreq[j]
                } else {
                    1. - block.afreq[j]
                };
                // not missing
                Zip::from(&mut sprob)
                    .and(&mut sprob_norecom)
                    .and(block.rhap.slice(s![j, ..]))
                    .apply(|p, p_norecom, &rhap| {
                        #[cfg(not(feature = "leak-resistant"))]
                        let emi = if tsym == rhap {
                            (1. - err) + err * afreq + BACKGROUND
                        } else {
                            err * afreq + BACKGROUND
                        };

                        #[cfg(feature = "leak-resistant")]
                        let emi = Real::select_from_4_f64(
                            tsym.tp_eq(&1),
                            tsym.tp_eq(&rhap),
                            (1. - err) + err * block.afreq[j] + BACKGROUND,
                            err * block.afreq[j] + BACKGROUND,
                            (1. - err) + err * (1. - block.afreq[j]) + BACKGROUND,
                            err * (1. - block.afreq[j]) + BACKGROUND,
                        );

                        *p *= emi;
                        *p_norecom *= emi;
                    });
            }

            let rec_real: Real = rec.into();

            // Transition
            let mut sprob_tot = sprob.iter().sum::<Real>() * (rec_real / m_real);
            sprob_norecom *= Real::from(1. - rec);
            let mut complement: Real = (1. - rec).into();

            // Lazy normalization (same as minimac)
            #[cfg(not(feature = "leak-resistant"))]
            if sprob_tot < NORM_THRESHOLD {
                sprob_tot *= NORM_SCALE_FACTOR;
                complement *= NORM_SCALE_FACTOR;
                sprob_norecom *= NORM_SCALE_FACTOR;
            }

            #[cfg(feature = "leak-resistant")]
            {
                let scale = sprob_tot
                    .tp_lt(&NORM_THRESHOLD)
                    .select(NORM_SCALE_FACTOR, Real::ONE);
                sprob_tot *= scale;
                complement *= scale;
                sprob_norecom *= scale;
            }

            sprob.assign(&(complement * &sprob + &block.clustsize * sprob_tot));

            // Impute very first position (edge case)
            // TODO fix this
            if b == 0 && j == 1 {
                //#[cfg(not(feature = "leak-resistant"))]
                //let combined = {
                //let x = &fwdprob_norecom.slice(s![0, ..]) * &sprob_norecom;
                //&jprob * &(x.clone() / (fwdprob_first * &sprob_first + E))
                //+ (&fwdprob.slice(s![0, ..]) * &sprob - x) / &block.clustsize
                //};

                //#[cfg(feature = "leak-resistant")]
                //let combined = {
                //let len = jprob.len();
                //Array1::from(
                //(0..len)
                //.map(|i| {
                //let x =
                //fwdprob_norecom.slice(s![0, ..])[i].safe_mul(sprob_norecom[i]);
                //jprob[i]
                //.safe_mul(x.safe_div(fwdprob_first[i] * sprob_first[i] + E))
                //.safe_add(
                //(fwdprob.slice(s![0, ..])[i]
                //.safe_mul(sprob[i])
                //.safe_sub(x))
                //.safe_div(block.clustsize[i]),
                //)
                //})
                //.collect::<Vec<Real>>(),
                //)
                //};

                //let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![0, ..])).fold(
                //(ZERO, ZERO),
                //|mut acc, &c, &rsym| {
                //#[cfg(not(feature = "leak-resistant"))]
                //if rsym == 1 {
                //acc.1 += c;
                //acc
                //} else {
                //acc.0 += c;
                //acc
                //}

                //#[cfg(feature = "leak-resistant")]
                //if rsym == 1 {
                //acc.1 = acc.1.safe_add(c);
                //acc
                //} else {
                //acc.0 = acc.0.safe_add(c);
                //acc
                //}
                //},
                //);

                //#[cfg(not(feature = "leak-resistant"))]
                //let res = p1 / (p1 + p0);

                //#[cfg(feature = "leak-resistant")]
                //let res = p1.safe_div(p1.safe_add(p0));

                //imputed[0] = res;

                imputed[0] = Real::NAN;
            }
        }

        let sprob_recom = &sprob - &sprob_norecom;

        //#[cfg(not(feature = "leak-resistant"))]
        //let sprob_norecom =
        //sprob_recom
        //.into_iter()
        //.map(|p| p.max(0.))
        //.collect::<Vec<_>>();

        // Unfold probabilities
        if b > 0 {
            Zip::from(&mut sprob_all)
                .and(&block.indmap)
                .apply(|p, &ui| {
                    *p = (sprob_recom[ui] / block.clustsize[ui])
                        + (*p * (sprob_norecom[ui] / (sprob_first[ui] + E)));
                });
        }

        var_offset += block.nvar - 1;
    }
    imputed
}
