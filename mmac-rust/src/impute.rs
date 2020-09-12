use crate::cache::*;
use crate::ref_panel::{Block, RefPanel};
use crate::{Input, Real};
use bitvec::prelude::BitSlice;
use lazy_static::lazy_static;
use ndarray::{s, Array1, Array2, ArrayView1, ArrayViewMut1, Zip};
use std::convert::TryFrom;

//type Cache<T> = LocalCacheSaver<T>;
type Cache<T> = FileCacheSaver<T>;

#[cfg(feature = "leak-resistant")]
mod leak_resistant_mod {
    pub use crate::bacc::Bacc;
    pub use timing_shield::{TpEq, TpOrd};
}

#[cfg(feature = "leak-resistant")]
use leak_resistant_mod::*;

const BACKGROUND: f32 = 1e-5;
const ERR: f32 = 0.00999;
const __NORM_THRESHOLD: f32 = 1e-20;
const __NORM_SCALE_FACTOR: f32 = 1e10;
const __E: f32 = 1e-30;

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
            sprob[ind as usize] += p;
        }
        sprob
    }

    #[cfg(feature = "leak-resistant")]
    {
        let mut sprob = vec![Bacc::init(); block.nuniq];
        for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
            sprob[ind as usize] += p;
        }
        Array1::from(sprob.into_iter().map(|v| v.result()).collect::<Vec<Real>>())
    }
}

fn single_emission(tsym: Input, block_afreq: f32, rhap: i8) -> Real {
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
    Real::select_from_4_f32(
        tsym.tp_eq(&1),
        tsym.tp_eq(&rhap),
        (1. - ERR) + ERR * block_afreq + BACKGROUND,
        ERR * block_afreq + BACKGROUND,
        (1. - ERR) + ERR * (1. - block_afreq) + BACKGROUND,
        ERR * (1. - block_afreq) + BACKGROUND,
    )
}

fn first_emission(tsym: Input, block: &Block, mut sprob_all: ArrayViewMut1<Real>) {
    #[cfg(not(feature = "leak-resistant"))]
    let cond = tsym != -1;

    #[cfg(feature = "leak-resistant")]
    // TODO: fix this leakage
    let cond = tsym.expose() != -1;

    if cond {
        let afreq = block.afreq[0];
        Zip::from(&mut sprob_all).and(&block.indmap).apply(|p, &i| {
            let emi = single_emission(tsym, afreq, block.rhap[0][i as usize] as i8);
            *p = emi;
        });
    }
}

fn later_emission(
    tsym: Input,
    mut sprob: ArrayViewMut1<Real>,
    mut sprob_norecom: ArrayViewMut1<Real>,
    block_afreq: f32,
    rhap_row: &BitSlice,
) {
    #[cfg(not(feature = "leak-resistant"))]
    let cond = tsym != -1;

    // TODO: fix this leakage
    #[cfg(feature = "leak-resistant")]
    let cond = tsym.expose() != -1;

    if cond {
        sprob
            .iter_mut()
            .zip(sprob_norecom.iter_mut())
            .zip(rhap_row.iter())
            .for_each(|((p, p_norecom), &rhap)| {
                let emi = single_emission(tsym, block_afreq, rhap as i8);
                *p *= emi;
                *p_norecom *= emi;
            });
    }
}

/// Lazy normalization (same as minimac)
#[allow(non_snake_case)]
fn normalize(sprob_tot: &mut Real, complement: &mut Real, mut sprob_norecom: ArrayViewMut1<Real>) {
    #[cfg(not(feature = "leak-resistant"))]
    {
        let NORM_THRESHOLD = *_NORM_THRESHOLD;
        let NORM_SCALE_FACTOR = *_NORM_SCALE_FACTOR;

        if *sprob_tot < NORM_THRESHOLD {
            *sprob_tot *= NORM_SCALE_FACTOR;
            *complement *= NORM_SCALE_FACTOR;
            sprob_norecom *= NORM_SCALE_FACTOR;
        }
    }

    #[cfg(feature = "leak-resistant")]
    {
        // no need for lazy normalization in log domain
        *complement /= *sprob_tot;
        sprob_norecom /= *sprob_tot;
        *sprob_tot = Real::ONE;
    }
}

fn transition(
    rec: f32,
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
            let ui = ui as usize;
            *p = (sprob_recom[ui] / block.clustsize[ui])
                + (*p * (sprob_norecom[ui] / (sprob_first[ui] + E)));
        });
}

#[allow(non_snake_case)]
fn impute(
    jprob: ArrayView1<Real>,
    clustsize: ArrayView1<Real>,
    rhap_row: &BitSlice,
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
    let (p0, p1) = combined
        .iter()
        .zip(rhap_row.iter())
        .fold((0., 0.), |mut acc, (&c, &rsym)| {
            if rsym {
                acc.1 += c;
                acc
            } else {
                acc.0 += c;
                acc
            }
        });

    #[cfg(feature = "leak-resistant")]
    let (p0, p1) = {
        let (p0, p1) = combined.iter().zip(rhap_row.iter()).fold(
            (Bacc::init(), Bacc::init()),
            |mut acc, (&c, &rsym)| {
                if rsym {
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
    mut ref_panel: RefPanel,
) -> Array1<Real> {
    assert!(thap.len() == ref_panel.n_markers);

    let blocks = &mut ref_panel.blocks;
    let m = ref_panel.n_haps;
    let m_real: Real = u16::try_from(m).unwrap().into();

    let cache_bound = 50;
    let mut fwdcache = Cache::new(cache_bound);
    let mut fwdcache_norecom = Cache::new(cache_bound);
    let mut fwdcache_first = Cache::new(cache_bound);
    let mut fwdcache_all = Cache::new(cache_bound);

    // Forward pass
    let mut sprob_all = Array1::<Real>::ones(m); // unnormalized
    let mut var_offset: usize = 0;

    // First position emission (edge case)
    first_emission(thap[0], &blocks[0], sprob_all.view_mut());

    for b in 0..blocks.len() {
        let block = &blocks[b];

        fwdcache_all.push(sprob_all.clone());

        let mut fwdprob = unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };
        let mut fwdprob_norecom =
            unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };

        let mut sprob = fold_probabilities(sprob_all.view(), &block);

        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        // Cache forward probabilities at first position
        fwdprob.row_mut(0).assign(&sprob);
        fwdprob_norecom.row_mut(0).assign(&sprob_norecom);

        // Walk
        Zip::from(block.rprob.slice(s![..block.nvar - 1]))
            .and(thap.slice(s![var_offset + 1..var_offset + block.nvar]))
            .and(block.afreq.slice(s![1..]))
            .and(&block.rhap[1..])
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

                    later_emission(
                        tsym,
                        sprob.view_mut(),
                        sprob_norecom.view_mut(),
                        block_afreq,
                        rhap_row,
                    );

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

    let mut fwdcache = fwdcache.into_retriever();
    let mut fwdcache_norecom = fwdcache_norecom.into_retriever();
    let mut fwdcache_first = fwdcache_first.into_retriever();
    let mut fwdcache_all = fwdcache_all.into_retriever();

    let mut imputed = unsafe { Array1::<Real>::uninitialized(thap.len()) };

    // Backward pass
    let mut sprob_all = Array1::<Real>::ones(m);
    let mut var_offset: usize = 0;
    for b in (0..blocks.len()).rev() {
        let block = blocks.pop().unwrap();
        let fwdprob = fwdcache.pop().unwrap();
        let fwdprob_norecom = fwdcache_norecom.pop().unwrap();
        let fwdprob_first = fwdcache_first.pop().unwrap();
        let fwdprob_all = fwdcache_all.pop().unwrap();

        // Precompute joint fwd-bwd term for imputation;
        // same as "Constants" variable in minimac
        #[cfg(not(feature = "leak-resistant"))]
        let jprob = {
            let mut jprob = Array1::<Real>::zeros(block.nuniq);
            Zip::from(&block.indmap)
                .and(&fwdprob_all)
                .and(&sprob_all)
                .apply(|&ind, &c, &p| {
                    jprob[ind as usize] += c * p;
                });
            jprob
        };

        #[cfg(feature = "leak-resistant")]
        let jprob = {
            let mut jprob = vec![Bacc::init(); block.nuniq];
            for ((&ind, &c), &p) in block.indmap.iter().zip(&fwdprob_all).zip(sprob_all.iter()) {
                jprob[ind as usize] += c * p;
            }
            Array1::from(jprob.into_iter().map(|v| v.result()).collect::<Vec<Real>>())
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
                block.rhap[j].as_bitslice(),
                fwdprob.slice(s![j, ..]),
                fwdprob_first.view(),
                fwdprob_norecom.slice(s![j, ..]),
                sprob.view(),
                sprob_first.view(),
                sprob_norecom.view(),
            );

            later_emission(
                thap[varind],
                sprob.view_mut(),
                sprob_norecom.view_mut(),
                block.afreq[j],
                block.rhap[j].as_bitslice(),
            );

            transition(
                rec,
                m_real,
                block.clustsize.view(),
                sprob.view_mut(),
                sprob_norecom.view_mut(),
            );

            // Impute very first position (edge case)
            if b == 0 && j == 1 {
                imputed[0] = impute(
                    jprob.view(),
                    block.clustsize.view(),
                    block.rhap[0].as_bitslice(),
                    fwdprob.slice(s![0, ..]),
                    fwdprob_first.view(),
                    fwdprob_norecom.slice(s![0, ..]),
                    sprob.view(),
                    sprob_first.view(),
                    sprob_norecom.view(),
                );
            }
        }

        let sprob_recom = &sprob - &sprob_norecom;

        if b > 0 {
            unfold_probabilities(
                &block,
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
