use crate::ref_panel::RefPanel;
use crate::{Input, Real};
use lazy_static::lazy_static;
use ndarray::{s, Array1, Array2, ArrayView1, Zip};
use std::convert::TryFrom;
#[cfg(feature = "leak-resistant")]
use timing_shield::{TpBool, TpEq, TpU64};

pub const BACKGROUND: f64 = 1e-5;

#[cfg(not(feature = "leak-resistant"))]
mod cons {
    pub const __NORM_THRESHOLD: f64 = 1e-20;
    pub const __NORM_SCALE_FACTOR: f64 = 1e10;
    pub const __ZERO: f64 = 0f64;
    pub const __E: f64 = 1e-30;
}

#[cfg(feature = "leak-resistant")]
mod cons {
    use super::Real;
    pub const __NORM_THRESHOLD: f64 = 1e-20;
    pub const __NORM_SCALE_FACTOR: f64 = 1e10;
    pub const __ZERO: Real = Real::ZERO;
    pub const __E: f64 = 1e-30;
}

lazy_static! {
    static ref _NORM_THRESHOLD: Real = cons::__NORM_THRESHOLD.into();
    static ref _NORM_SCALE_FACTOR: Real = cons::__NORM_SCALE_FACTOR.into();
    static ref _E: Real = cons::__E.into();
}

#[inline]
#[cfg(feature = "leak-resistant")]
fn const_time_select(cond: TpBool, a: f64, b: f64) -> f64 {
    #[inline]
    fn f64_to_u64(x: f64) -> u64 {
        unsafe { *(&x as *const f64 as *const u64) }
    }
    #[inline]
    fn u64_to_f64(x: u64) -> f64 {
        unsafe { *(&x as *const u64 as *const f64) }
    }

    u64_to_f64(
        cond.select(TpU64::protect(f64_to_u64(a)), TpU64::protect(f64_to_u64(b)))
            .expose(),
    )
}

#[allow(non_snake_case)]
pub fn impute_chunk(
    _chunk_id: usize,
    thap: ArrayView1<Input>,
    ref_panel: &RefPanel,
) -> Array1<Real> {
    assert!(thap.len() == ref_panel.n_markers);

    // Put all constants on stack
    let ZERO = cons::__ZERO;
    let NORM_THRESHOLD = *_NORM_THRESHOLD;
    let NORM_SCALE_FACTOR = *_NORM_SCALE_FACTOR;
    let E = *_E;

    let blocks = &ref_panel.blocks;
    let m = ref_panel.n_haps;
    let m_real: Real = u32::try_from(m).unwrap().into();

    let mut imputed = Array1::<Real>::zeros(thap.len());

    let mut fwdcache = Vec::new();
    let mut fwdcache_norecom = Vec::new();
    let mut fwdcache_first = Vec::new();
    let mut fwdcache_all = Array2::<Real>::zeros((blocks.len(), m));

    /* Forward pass */
    let mut sprob_all = Array1::<Real>::ones(m); // unnormalized
    let mut var_offset: usize = 0;

    // First position emission (edge case)
    // TODO: fix this leakage
    #[cfg(not(feature = "leak-resistant"))]
    let cond = thap[0] != -1;
    #[cfg(feature = "leak-resistant")]
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

        #[cfg(feature = "leak-resistant")]
        let afreq = const_time_select(tsym.tp_eq(&1), block.afreq[0], 1. - block.afreq[0]);

        Zip::from(&mut sprob_all)
            .and(&block.indmap)
            .apply(|p, &ind| {
                #[cfg(not(feature = "leak-resistant"))]
                let emi = if tsym == block.rhap[[0, ind]] {
                    (1. - err) + err * afreq + BACKGROUND
                } else {
                    err * afreq + BACKGROUND
                };

                #[cfg(feature = "leak-resistant")]
                let emi = const_time_select(
                    tsym.tp_eq(&block.rhap[[0, ind]]),
                    (1. - err) + err * afreq + BACKGROUND,
                    err * afreq + BACKGROUND,
                );

                *p = emi.into();
            });
    }

    for b in 0..blocks.len() {
        let block = &blocks[b];

        fwdcache_all.slice_mut(s![b, ..]).assign(&sprob_all); // save cache

        let mut fwdprob = Array2::<Real>::zeros((block.nvar, block.nuniq));
        let mut fwdprob_norecom = Array2::<Real>::zeros((block.nvar, block.nuniq));

        // Fold probabilities
        let mut sprob = Array1::<Real>::zeros(block.nuniq);
        for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
            sprob[ind] += p;
        }

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

                    #[cfg(not(feature = "leak-resistant"))]
                    let afreq = if tsym == 1 {
                        block_afreq
                    } else {
                        1. - block_afreq
                    };

                    #[cfg(feature = "leak-resistant")]
                    let afreq = const_time_select(tsym.tp_eq(&1), block_afreq, 1. - block_afreq);

                    let rec_real: Real = rec.into();

                    // Transition
                    let mut sprob_tot = sprob.iter().sum::<Real>() * (rec_real / m_real);
                    sprob_norecom *= Real::from(1. - rec);
                    let mut complement: Real = (1. - rec).into();

                    // Lazy normalization (same as minimac)
                    if sprob_tot < NORM_THRESHOLD {
                        sprob_tot *= NORM_SCALE_FACTOR;
                        complement *= NORM_SCALE_FACTOR;
                        sprob_norecom *= NORM_SCALE_FACTOR;
                    }

                    sprob.assign(&(complement * &sprob + &block.clustsize * sprob_tot));

                    // Emission
                    // TODO: fix this leakage
                    #[cfg(not(feature = "leak-resistant"))]
                    let cond = tsym != -1;

                    #[cfg(feature = "leak-resistant")]
                    let cond = tsym.expose() != -1;

                    if cond {
                        Zip::from(&mut sprob)
                            .and(&mut sprob_norecom)
                            .and(&rhap_row)
                            .apply(|p, p_norecom, &rhap| {
                                #[cfg(not(feature = "leak-resistant"))]
                                let emi: Real = if tsym == rhap {
                                    (1. - err) + err * afreq + BACKGROUND
                                } else {
                                    err * afreq + BACKGROUND
                                }
                                .into();

                                #[cfg(feature = "leak-resistant")]
                                let emi: Real = const_time_select(
                                    tsym.tp_eq(&rhap),
                                    (1. - err) + err * afreq + BACKGROUND,
                                    err * afreq + BACKGROUND,
                                )
                                .into();

                                *p *= emi;
                                *p_norecom *= emi;
                            });
                    }

                    // Cache forward probabilities
                    fwdprob_row.assign(&sprob);
                    fwdprob_norecom_row.assign(&sprob_norecom);
                },
            );

        let mut sprob_recom = &sprob - &sprob_norecom;
        sprob_recom.iter_mut().for_each(|p| *p = (*p).max(ZERO));

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
        let mut jprob = Array1::<Real>::zeros(block.nuniq);
        Zip::from(&block.indmap)
            .and(fwdcache_all.slice(s![b, ..]))
            .and(&sprob_all)
            .apply(|&ind, &c, &p| {
                jprob[ind] += c * p;
            });

        // Fold probabilities
        let mut sprob = Array1::<Real>::zeros(block.nuniq);
        for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
            sprob[ind] += p;
        }

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

            #[cfg(not(feature = "leak-resistant"))]
            let afreq = if tsym == 1 {
                block.afreq[j]
            } else {
                1. - block.afreq[j]
            };

            #[cfg(feature = "leak-resistant")]
            let afreq = const_time_select(tsym.tp_eq(&1), block.afreq[j], 1. - block.afreq[j]);

            // Impute
            let combined = &jprob
                * &(&fwdprob_norecom.slice(s![j, ..]) * &sprob_norecom
                    / (fwdprob_first * &sprob_first + E))
                + (&fwdprob.slice(s![j, ..]) * &sprob
                    - &fwdprob_norecom.slice(s![j, ..]) * &sprob_norecom)
                    / &block.clustsize;

            let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![j, ..])).fold(
                (ZERO, ZERO),
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

            imputed[varind] = p1 / (p1 + p0);

            // Emission
            // TODO: fix this leakage
            #[cfg(not(feature = "leak-resistant"))]
            let cond = tsym != -1;

            #[cfg(feature = "leak-resistant")]
            let cond = tsym.expose() != -1;

            if cond {
                // not missing
                Zip::from(&mut sprob)
                    .and(&mut sprob_norecom)
                    .and(block.rhap.slice(s![j, ..]))
                    .apply(|p, p_norecom, &rhap| {
                        #[cfg(not(feature = "leak-resistant"))]
                        let emi: Real = if tsym == rhap {
                            (1. - err) + err * afreq + BACKGROUND
                        } else {
                            err * afreq + BACKGROUND
                        }
                        .into();

                        #[cfg(feature = "leak-resistant")]
                        let emi = const_time_select(
                            tsym.tp_eq(&rhap),
                            (1. - err) + err * afreq + BACKGROUND,
                            err * afreq + BACKGROUND,
                        )
                        .into();

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
            if sprob_tot < NORM_THRESHOLD {
                sprob_tot *= NORM_SCALE_FACTOR;
                complement *= NORM_SCALE_FACTOR;
                sprob_norecom *= NORM_SCALE_FACTOR;
            }

            sprob.assign(&(complement * &sprob + &block.clustsize * sprob_tot));

            // Impute very first position (edge case)
            if b == 0 && j == 1 {
                let combined = &jprob
                    * &(&fwdprob_norecom.slice(s![0, ..]) * &sprob_norecom
                        / (fwdprob_first * &sprob_first + E))
                    + (&fwdprob.slice(s![0, ..]) * &sprob
                        - &fwdprob_norecom.slice(s![0, ..]) * &sprob_norecom)
                        / &block.clustsize;

                let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![0, ..])).fold(
                    (ZERO, ZERO),
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

                imputed[0] = p1 / (p1 + p0);
            }
        }

        let mut sprob_recom = &sprob - &sprob_norecom;
        sprob_recom.iter_mut().for_each(|p| *p = (*p).max(ZERO));

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
