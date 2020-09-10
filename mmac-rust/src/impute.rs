use crate::ref_panel::RefPanel;
use crate::Block;
use crate::{Input, Real};
use lazy_static::lazy_static;
use ndarray::{s, Array1, Array2, ArrayView1, ArrayViewMut1, Zip};
use std::convert::TryFrom;

#[cfg(feature = "leak-resistant")]
mod leak_resistant_mod {
    pub use crate::bacc::Bacc;
    pub use timing_shield::{TpEq, TpOrd};
}

#[cfg(feature = "leak-resistant")]
use leak_resistant_mod::*;

const BACKGROUND: f64 = 1e-5;
const ERR: f64 = 0.00999;
const __NORM_THRESHOLD: f64 = 1e-20;
const __NORM_SCALE_FACTOR: f64 = 1e10;
const __E: f64 = 1e-30;

lazy_static! {
    static ref _NORM_THRESHOLD: Real = __NORM_THRESHOLD.into();
    static ref _NORM_SCALE_FACTOR: Real = __NORM_SCALE_FACTOR.into();
    static ref _E: Real = __E.into();
}

fn fold_probabilities(sprob_all: ArrayView1<Real>, block: &Block) -> Array1<Real> {
    #[cfg(not(feature = "leak-resistant"))]
    {
        let mut sprob = Array1::<Real>::zeros(block.nuniq);
        for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
            sprob[ind] += p;
        }
        sprob
    }

    #[cfg(feature = "leak-resistant")]
    {
        let sprob = (0..block.nuniq)
            .map(|i| {
                block.rev_indmap[&i]
                    .iter()
                    .map(|&j| sprob_all[j])
                    .sum::<Real>()
            })
            .collect::<Vec<Real>>();
        Array1::from(sprob)
    }
}

fn single_emission(tsym: Input, block_afreq: f64, rhap: i8) -> Real {
    #[cfg(not(feature = "leak-resistant"))]
    {
        let afreq = if tsym == 1 {
            block_afreq
        } else {
            1. - block_afreq
        };
        if tsym == rhap {
            (1. - ERR) + ERR * afreq + BACKGROUND
        } else {
            ERR * afreq + BACKGROUND
        }
    }

    #[cfg(feature = "leak-resistant")]
    Real::select_from_4_f64(
        tsym.tp_eq(&1),
        tsym.tp_eq(&rhap),
        (1. - ERR) + ERR * block_afreq + BACKGROUND,
        ERR * block_afreq + BACKGROUND,
        (1. - ERR) + ERR * (1. - block_afreq) + BACKGROUND,
        ERR * (1. - block_afreq) + BACKGROUND,
    )
}

fn first_emission(tsym: Input, block: &Block, mut sprob_all: ArrayViewMut1<Real>) {
    let afreq = block.afreq[0];
    Zip::from(&mut sprob_all).and(&block.indmap).apply(|p, &i| {
        let emi = single_emission(tsym, afreq, block.rhap[[0, i]]);
        *p = emi;
    });
}

fn later_emission(
    tsym: Input,
    mut sprob: ArrayViewMut1<Real>,
    mut sprob_norecom: ArrayViewMut1<Real>,
    block_afreq: f64,
    rhap_row: ArrayView1<i8>,
) {
    Zip::from(&mut sprob)
        .and(&mut sprob_norecom)
        .and(rhap_row)
        .apply(|p, p_norecom, &rhap| {
            let emi = single_emission(tsym, block_afreq, rhap);
            *p *= emi;
            *p_norecom *= emi;
        });
}

/// Lazy normalization (same as minimac)
#[allow(non_snake_case)]
fn normalize(sprob_tot: &mut Real, complement: &mut Real, mut sprob_norecom: ArrayViewMut1<Real>) {
    let NORM_THRESHOLD = *_NORM_THRESHOLD;
    let NORM_SCALE_FACTOR = *_NORM_SCALE_FACTOR;

    #[cfg(not(feature = "leak-resistant"))]
    if *sprob_tot < NORM_THRESHOLD {
        *sprob_tot *= NORM_SCALE_FACTOR;
        *complement *= NORM_SCALE_FACTOR;
        sprob_norecom *= NORM_SCALE_FACTOR;
    }

    #[cfg(feature = "leak-resistant")]
    {
        let scale = sprob_tot
            .tp_lt(&NORM_THRESHOLD)
            .select(NORM_SCALE_FACTOR, Real::ONE);
        *sprob_tot *= scale;
        *complement *= scale;
        sprob_norecom *= scale;
    }
}

fn transition(
    rec: f64,
    m_real: Real,
    clustsize: ArrayView1<Real>,
    mut sprob: ArrayViewMut1<Real>,
    mut sprob_norecom: ArrayViewMut1<Real>,
) {
    let rec_real: Real = rec.into();

    let mut sprob_tot = sprob.iter().sum::<Real>() * (rec_real / m_real);
    sprob_norecom *= Real::from(1. - rec);
    let mut complement: Real = (1. - rec).into();

    normalize(&mut sprob_tot, &mut complement, sprob_norecom.view_mut());

    sprob.assign(&(complement * &sprob + &clustsize * sprob_tot));
}

#[allow(non_snake_case)]
fn unfold_probabilities(
    block: &Block,
    mut sprob_all: ArrayViewMut1<Real>,
    sprob_first: ArrayView1<Real>,
    sprob_recom: ArrayView1<Real>,
    sprob_norecom: ArrayView1<Real>,
) {
    let E = *_E;
    Zip::from(&mut sprob_all)
        .and(&block.indmap)
        .apply(|p, &ui| {
            *p = (sprob_recom[ui] / block.clustsize[ui])
                + (*p * (sprob_norecom[ui] / (sprob_first[ui] + E)));
        });
}

#[allow(non_snake_case)]
fn impute(
    jprob: ArrayView1<Real>,
    clustsize: ArrayView1<Real>,
    rhap_row: ArrayView1<i8>,
    fwdprob_row: ArrayView1<Real>,
    fwdprob_first: ArrayView1<Real>,
    fwdprob_norecom_row: ArrayView1<Real>,
    sprob: ArrayView1<Real>,
    sprob_first: ArrayView1<Real>,
    sprob_norecom: ArrayView1<Real>,
) -> Real {
    let E = *_E;
    let combined = {
        let x = &fwdprob_norecom_row * &sprob_norecom;
        &jprob * &(x.clone() / (&fwdprob_first * &sprob_first + E))
            + (&fwdprob_row * &sprob - x) / &clustsize
    };

    #[cfg(not(feature = "leak-resistant"))]
    let (p0, p1) = Zip::from(&combined)
        .and(rhap_row)
        .fold((0., 0.), |mut acc, &c, &rsym| {
            if rsym == 1 {
                acc.1 += c;
                acc
            } else {
                acc.0 += c;
                acc
            }
        });

    #[cfg(feature = "leak-resistant")]
    let (p0, p1) = {
        let (p0, p1) = Zip::from(&combined).and(rhap_row).fold(
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

    p1 / (p1 + p0)
}

pub fn impute_chunk(
    _chunk_id: usize,
    thap: ArrayView1<Input>,
    ref_panel: &RefPanel,
) -> Array1<Real> {
    assert!(thap.len() == ref_panel.n_markers);

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
    let tsym = thap[0];
    #[cfg(not(feature = "leak-resistant"))]
    let cond = tsym != -1;

    #[cfg(feature = "leak-resistant")]
    // TODO: fix this leakage
    let cond = tsym.expose() != -1;

    if cond {
        first_emission(tsym, &blocks[0], sprob_all.view_mut());
    }

    for b in 0..blocks.len() {
        let block = &blocks[b];

        fwdcache_all.slice_mut(s![b, ..]).assign(&sprob_all); // save cache

        let mut fwdprob = unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };
        let mut fwdprob_norecom =
            unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };

        let mut sprob = fold_probabilities(sprob_all.view(), &block);

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
                    transition(
                        rec,
                        m_real,
                        block.clustsize.view(),
                        sprob.view_mut(),
                        sprob_norecom.view_mut(),
                    );

                    #[cfg(not(feature = "leak-resistant"))]
                    let cond = tsym != -1;

                    // TODO: fix this leakage
                    #[cfg(feature = "leak-resistant")]
                    let cond = tsym.expose() != -1;

                    if cond {
                        later_emission(
                            tsym,
                            sprob.view_mut(),
                            sprob_norecom.view_mut(),
                            block_afreq,
                            rhap_row,
                        );
                    }

                    // Cache forward probabilities
                    fwdprob_row.assign(&sprob);
                    fwdprob_norecom_row.assign(&sprob_norecom);
                },
            );

        let sprob_recom = &sprob - &sprob_norecom;

        // Skip last block
        if b < blocks.len() - 1 {
            unfold_probabilities(
                block,
                sprob_all.view_mut(),
                sprob_first.view(),
                sprob_recom.view(),
                sprob_norecom.view(),
            );
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

        let mut sprob = fold_probabilities(sprob_all.view(), &block);
        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        // Walk
        for j in (1..block.nvar).rev() {
            let rec = block.rprob[j - 1];
            let varind = thap.len() - (var_offset + (block.nvar - j));

            imputed[varind] = impute(
                jprob.view(),
                block.clustsize.view(),
                block.rhap.slice(s![j, ..]),
                fwdprob.slice(s![j, ..]),
                fwdprob_first.view(),
                fwdprob_norecom.slice(s![j, ..]),
                sprob.view(),
                sprob_first.view(),
                sprob_norecom.view(),
            );

            let tsym = thap[varind];
            #[cfg(not(feature = "leak-resistant"))]
            let cond = tsym != -1;

            // TODO: fix this leakage
            #[cfg(feature = "leak-resistant")]
            let cond = tsym.expose() != -1;

            if cond {
                later_emission(
                    tsym,
                    sprob.view_mut(),
                    sprob_norecom.view_mut(),
                    block.afreq[j],
                    block.rhap.slice(s![j, ..]),
                );
            }

            transition(
                rec,
                m_real,
                block.clustsize.view(),
                sprob.view_mut(),
                sprob_norecom.view_mut(),
            );

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

        if b > 0 {
            unfold_probabilities(
                block,
                sprob_all.view_mut(),
                sprob_first.view(),
                sprob_recom.view(),
                sprob_norecom.view(),
            );
        }

        var_offset += block.nvar - 1;
    }
    imputed
}
