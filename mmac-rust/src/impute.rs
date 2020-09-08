#[cfg(feature = "leak-resistant")]
use crate::const_time;
use crate::ref_panel::RefPanel;
use crate::{Input, Real};
use lazy_static::lazy_static;
use ndarray::{s, Array1, Array2, ArrayView1, Zip};
use std::convert::TryFrom;
#[cfg(feature = "leak-resistant")]
use timing_shield::TpEq;

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

// Balanced accumulator
#[cfg(feature = "leak-resistant")]
pub struct Bacc(Vec<Option<Real>>);

#[cfg(feature = "leak-resistant")]
impl Bacc {
    pub fn new() -> Bacc {
        Bacc(Vec::new())
    }

    pub fn add(&mut self, val: Real) {
        let mut val = Some(val);
        for slot in self.0.iter_mut() {
            if slot.is_some() {
                val.replace(slot.take().unwrap() + val.unwrap());
            } else {
                slot.replace(val.take().unwrap());
                break;
            }
        }

        if val.is_some() {
            self.0.push(val);
        }
    }

    pub fn result(self) -> Real {
        self.0
            .into_iter()
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .sum()
    }
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
                let emi: Real = const_time::select_4_no_ln(
                    tsym.tp_eq(&1),
                    tsym.tp_eq(&block.rhap[[0, ind]]),
                    ((1. - err) + err * block.afreq[0] + BACKGROUND).ln(),
                    (err * block.afreq[0] + BACKGROUND).ln(),
                    ((1. - err) + err * (1. - block.afreq[0]) + BACKGROUND).ln(),
                    (err * (1. - block.afreq[0]) + BACKGROUND).ln(),
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
                                let emi: Real = if tsym == rhap {
                                    (1. - err) + err * afreq + BACKGROUND
                                } else {
                                    err * afreq + BACKGROUND
                                }
                                .into();

                                #[cfg(feature = "leak-resistant")]
                                let emi: Real = const_time::select_4_no_ln(
                                    tsym.tp_eq(&1),
                                    tsym.tp_eq(&rhap),
                                    ((1. - err) + err * block_afreq + BACKGROUND).ln(),
                                    (err * block_afreq + BACKGROUND).ln(),
                                    ((1. - err) + err * (1. - block_afreq) + BACKGROUND).ln(),
                                    (err * (1. - block_afreq) + BACKGROUND).ln(),
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

            #[cfg(feature = "leak-resistant")]
            let (p0, p1) = {
                let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![j, ..])).fold(
                    (Bacc::new(), Bacc::new()),
                    |mut acc, &c, &rsym| {
                        if rsym == 1 {
                            acc.1.add(c);
                            acc
                        } else {
                            acc.0.add(c);
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
                        let emi: Real = if tsym == rhap {
                            (1. - err) + err * afreq + BACKGROUND
                        } else {
                            err * afreq + BACKGROUND
                        }
                        .into();

                        #[cfg(feature = "leak-resistant")]
                        let emi: Real = const_time::select_4_no_ln(
                            tsym.tp_eq(&1),
                            tsym.tp_eq(&rhap),
                            ((1. - err) + err * block.afreq[j] + BACKGROUND).ln(),
                            (err * block.afreq[j] + BACKGROUND).ln(),
                            ((1. - err) + err * (1. - block.afreq[j]) + BACKGROUND).ln(),
                            (err * (1. - block.afreq[j]) + BACKGROUND).ln(),
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
            if sprob_tot < NORM_THRESHOLD {
                sprob_tot *= NORM_SCALE_FACTOR;
                complement *= NORM_SCALE_FACTOR;
                sprob_norecom *= NORM_SCALE_FACTOR;
            }

            sprob.assign(&(complement * &sprob + &block.clustsize * sprob_tot));

            // Impute very first position (edge case)
            if b == 0 && j == 1 {
                #[cfg(not(feature = "leak-resistant"))]
                let combined = {
                    let x = &fwdprob_norecom.slice(s![0, ..]) * &sprob_norecom;
                    &jprob * &(x.clone() / (fwdprob_first * &sprob_first + E))
                        + (&fwdprob.slice(s![0, ..]) * &sprob - x) / &block.clustsize
                };

                #[cfg(feature = "leak-resistant")]
                let combined = {
                    let len = jprob.len();
                    Array1::from(
                        (0..len)
                            .map(|i| {
                                let x =
                                    fwdprob_norecom.slice(s![0, ..])[i].safe_mul(sprob_norecom[i]);
                                jprob[i]
                                    .safe_mul(x.safe_div(fwdprob_first[i] * sprob_first[i] + E))
                                    .safe_add(
                                        (fwdprob.slice(s![0, ..])[i]
                                            .safe_mul(sprob[i])
                                            .safe_sub(x))
                                        .safe_div(block.clustsize[i]),
                                    )
                            })
                            .collect::<Vec<Real>>(),
                    )
                };

                let (p0, p1) = Zip::from(&combined).and(block.rhap.slice(s![0, ..])).fold(
                    (ZERO, ZERO),
                    |mut acc, &c, &rsym| {
                        #[cfg(not(feature = "leak-resistant"))]
                        if rsym == 1 {
                            acc.1 += c;
                            acc
                        } else {
                            acc.0 += c;
                            acc
                        }

                        #[cfg(feature = "leak-resistant")]
                        if rsym == 1 {
                            acc.1 = acc.1.safe_add(c);
                            acc
                        } else {
                            acc.0 = acc.0.safe_add(c);
                            acc
                        }
                    },
                );

                #[cfg(not(feature = "leak-resistant"))]
                let res = p1 / (p1 + p0);

                #[cfg(feature = "leak-resistant")]
                let res = p1.safe_div(p1.safe_add(p0));

                imputed[0] = res;
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
