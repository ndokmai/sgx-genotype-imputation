use crate::block::Block;
use crate::ref_panel::RefPanelMeta;
use crate::symbol::Symbol;
use crate::{Real, TargetSymbol};
use bitvec::slice::BitSlice;
use lazy_static::lazy_static;
use ndarray::{Array1, Array2, ArrayView1, ArrayViewMut1, Zip};
use rayon::prelude::*;
use std::convert::TryFrom;

#[cfg(feature = "leak-resistant")]
pub use timing_shield::{TpEq, TpI8, TpOrd};

const BACKGROUND: f32 = 1e-5;
const ERR: f32 = 0.00999;

#[cfg(not(feature = "leak-resistant"))]
const NORM_THRESHOLD: f32 = 1e-20;
#[cfg(not(feature = "leak-resistant"))]
const NORM_SCALE_FACTOR: f32 = 1e10;

const __E: f32 = 1e-30;

lazy_static! {
    static ref _E: Real = __E.into();
}

struct FwdCacheBlock {
    pub prob: Array2<Real>,
    pub prob_norecom: Array2<Real>,
    pub prob_first: Array1<Real>,
    pub prob_all: Array1<Real>,
}

pub fn smac_batch(
    ref_panel_meta: &RefPanelMeta,
    ref_panel_blocks: &[Block],
    bitmask: &[bool],
    symbols_batch: Vec<Vec<Symbol>>,
) -> Vec<Vec<Real>> {
    symbols_batch
        .into_par_iter()
        .map(|s| smac(ref_panel_meta, ref_panel_blocks, bitmask, s))
        .collect()
}

pub fn smac(
    ref_panel_meta: &RefPanelMeta,
    ref_panel_blocks: &[Block],
    bitmask: &[bool],
    symbols: Vec<Symbol>,
) -> Vec<Real> {
    assert_eq!(ref_panel_meta.n_blocks, ref_panel_blocks.len());
    assert_eq!(ref_panel_meta.n_markers, bitmask.len());
    assert_eq!(bitmask.iter().filter(|&&v| v).count(), symbols.len());

    #[cfg(feature = "leak-resistant")]
    let symbols = symbols
        .into_iter()
        .map(|v| TargetSymbol::protect(v.into()))
        .collect::<Vec<_>>();

    let fwd_cache = forward_pass(
        ref_panel_meta,
        ref_panel_blocks,
        bitmask,
        symbols.as_slice(),
    );

    backward_pass(
        ref_panel_meta,
        ref_panel_blocks,
        bitmask,
        symbols.as_slice(),
        fwd_cache,
    )
}

fn forward_pass(
    ref_panel_meta: &RefPanelMeta,
    ref_panel_blocks: &[Block],
    bitmask: &[bool],
    symbols: &[TargetSymbol],
) -> Vec<FwdCacheBlock> {
    let mut bitmask_iter = bitmask.iter().cloned();
    let mut symbol_iter = symbols.iter().cloned();

    let mut fwd_cache_blocks = Vec::with_capacity(ref_panel_blocks.len());

    let m = ref_panel_meta.n_haps;
    let m_real: Real = u16::try_from(m).unwrap().into();
    let mut sprob_all = Array1::<Real>::ones(m); // unnormalized

    for (b_id, block) in ref_panel_blocks.iter().enumerate() {
        // First position emission (edge case)
        if b_id == 0 {
            let mask = bitmask_iter.next().unwrap();
            if mask {
                let symbol = symbol_iter.next().unwrap();
                first_emission(symbol, block, sprob_all.view_mut());
            }
        }

        let prob_all_cache = sprob_all.clone();
        let mut prob_cache = unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };
        let mut prob_norecom_cache =
            unsafe { Array2::<Real>::uninitialized((block.nvar, block.nuniq)) };

        let mut sprob = fold_probabilities(sprob_all.view(), block);
        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        // Walk
        // Cache forward probabilities at first position
        prob_cache.row_mut(0).assign(&sprob);
        prob_norecom_cache.row_mut(0).assign(&sprob_norecom);

        for i in 1..block.nvar {
            let mask = bitmask_iter.next().unwrap();
            let rec = block.rprob[i - 1];

            transition(
                rec,
                m_real,
                block.clustsize.view(),
                sprob.view_mut(),
                sprob_norecom.view_mut(),
            );

            if mask {
                let symbol = symbol_iter.next().unwrap();

                later_emission(
                    symbol,
                    sprob.view_mut(),
                    sprob_norecom.view_mut(),
                    block.afreq[i],
                    block.rhap[i].as_bitslice(),
                );
            }

            prob_cache.row_mut(i).assign(&sprob);
            prob_norecom_cache.row_mut(i).assign(&sprob_norecom);
        }

        // Skip last block
        if b_id != ref_panel_blocks.len() - 1 {
            let sprob_recom = sprob - sprob_norecom.clone();
            unfold_probabilities(
                block,
                sprob_all.view_mut(),
                sprob_first.clone(),
                sprob_recom,
                sprob_norecom,
            );
        }

        let fwd_cache_block = FwdCacheBlock {
            prob: prob_cache,
            prob_norecom: prob_norecom_cache,
            prob_first: sprob_first,
            prob_all: prob_all_cache,
        };
        fwd_cache_blocks.push(fwd_cache_block);
    }
    fwd_cache_blocks
}

fn backward_pass(
    ref_panel_meta: &RefPanelMeta,
    ref_panel_blocks: &[Block],
    bitmask: &[bool],
    symbols: &[TargetSymbol],
    cache_blocks: Vec<FwdCacheBlock>,
) -> Vec<Real> {
    let mut bitmask_iter = bitmask.iter().cloned().rev();
    let mut symbol_iter = symbols.iter().cloned().rev();

    let mut outputs = Vec::with_capacity(ref_panel_blocks.len());

    let m = ref_panel_meta.n_haps;
    let m_real: Real = u16::try_from(m).unwrap().into();
    let mut sprob_all = Array1::<Real>::ones(m); // unnormalized

    for (b_id, (block, cache_block)) in ref_panel_blocks
        .iter()
        .zip(cache_blocks.into_iter())
        .enumerate()
        .rev()
    {
        let (fwd_prob, fwd_prob_norecom, fwd_prob_first, fwd_prob_all) = (
            cache_block.prob,
            cache_block.prob_norecom,
            cache_block.prob_first,
            cache_block.prob_all,
        );

        let mut sprob = fold_probabilities(sprob_all.view(), &block);
        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        let jprob = precompute_joint(block, sprob_all.clone(), fwd_prob_all);

        // Walk
        for j in (1..block.nvar).rev() {
            let output = impute(
                jprob.clone(),
                block.clustsize.view(),
                block.rhap[j].as_bitslice(),
                fwd_prob.row(j).to_owned(),
                fwd_prob_first.clone(),
                fwd_prob_norecom.row(j).to_owned(),
                sprob.clone(),
                sprob_first.clone(),
                sprob_norecom.clone(),
            );
            outputs.push(output);

            let mask = bitmask_iter.next().unwrap();
            if mask {
                let symbol = symbol_iter.next().unwrap();

                later_emission(
                    symbol,
                    sprob.view_mut(),
                    sprob_norecom.view_mut(),
                    block.afreq[j],
                    block.rhap[j].as_bitslice(),
                );
            }

            let rec = block.rprob[j - 1];
            transition(
                rec,
                m_real,
                block.clustsize.view(),
                sprob.view_mut(),
                sprob_norecom.view_mut(),
            );
        }

        if b_id == 0 {
            let output = impute(
                jprob,
                block.clustsize.view(),
                block.rhap[0].as_bitslice(),
                fwd_prob.row(0).to_owned(),
                fwd_prob_first,
                fwd_prob_norecom.row(0).to_owned(),
                sprob,
                sprob_first,
                sprob_norecom,
            );
            outputs.push(output);
        } else {
            let sprob_recom = sprob - sprob_norecom.clone();
            unfold_probabilities(
                &block,
                sprob_all.view_mut(),
                sprob_first,
                sprob_recom,
                sprob_norecom,
            );
        }
    }
    outputs.into_iter().rev().collect()
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
        let mut sprob = vec![Vec::with_capacity(20); block.nuniq];
        for (&ind, &p) in block.indmap.iter().zip(sprob_all.iter()) {
            sprob[ind as usize].push(p);
        }
        Array1::from(
            sprob
                .into_iter()
                .map(|mut v| Real::sum_in_place(v.as_mut_slice()))
                .collect::<Vec<Real>>(),
        )
    }
}

fn single_emission(tsym: TargetSymbol, block_afreq: f32, rhap: Symbol) -> Real {
    #[cfg(not(feature = "leak-resistant"))]
    {
        let afreq = if tsym == Symbol::Alt {
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
        tsym.tp_eq(&(rhap as i8)),
        (1. - ERR) + ERR * block_afreq + BACKGROUND,
        ERR * block_afreq + BACKGROUND,
        (1. - ERR) + ERR * (1. - block_afreq) + BACKGROUND,
        ERR * (1. - block_afreq) + BACKGROUND,
    )
}

fn first_emission(tsym: TargetSymbol, block: &Block, mut sprob_all: ArrayViewMut1<Real>) {
    let afreq = block.afreq[0];

    #[cfg(not(feature = "leak-resistant"))]
    if tsym != Symbol::Missing {
        Zip::from(&mut sprob_all)
            .and(&block.indmap)
            .apply(|prob, &i| {
                let emi = single_emission(tsym, afreq, block.rhap[0][i as usize].into());
                *prob *= emi
            });
    }

    #[cfg(feature = "leak-resistant")]
    {
        let cond = tsym.tp_not_eq(&-1);
        Zip::from(&mut sprob_all)
            .and(&block.indmap)
            .apply(|prob, &i| {
                let emi = single_emission(tsym, afreq, block.rhap[0][i as usize].into());
                *prob = cond.select(emi, *prob);
            });
    }
}

fn later_emission(
    tsym: TargetSymbol,
    mut sprob: ArrayViewMut1<Real>,
    mut sprob_norecom: ArrayViewMut1<Real>,
    block_afreq: f32,
    rhap_row: &BitSlice,
) {
    #[cfg(not(feature = "leak-resistant"))]
    if tsym != Symbol::Missing {
        sprob
            .iter_mut()
            .zip(sprob_norecom.iter_mut())
            .zip(rhap_row.iter())
            .for_each(|((prob, prob_norecom), rhap)| {
                let emi = single_emission(tsym, block_afreq, (*rhap).into());
                *prob *= emi;
                *prob_norecom *= emi;
            });
    }
    #[cfg(feature = "leak-resistant")]
    {
        let cond = tsym.tp_not_eq(&-1);
        sprob
            .iter_mut()
            .zip(sprob_norecom.iter_mut())
            .zip(rhap_row.iter())
            .for_each(|((prob, prob_norecom), rhap)| {
                let emi = single_emission(tsym, block_afreq, (*rhap).into());
                *prob = cond.select(*prob * emi, *prob);
                *prob_norecom = cond.select(*prob_norecom * emi, *prob_norecom);
            });
    }
}

/// Lazy normalization (same as minimac)
#[allow(non_snake_case)]
fn normalize(sprob_tot: &mut Real, complement: &mut Real, mut sprob_norecom: ArrayViewMut1<Real>) {
    #[cfg(not(feature = "leak-resistant"))]
    if *sprob_tot < NORM_THRESHOLD {
        *sprob_tot *= NORM_SCALE_FACTOR;
        *complement *= NORM_SCALE_FACTOR;
        sprob_norecom *= NORM_SCALE_FACTOR;
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

fn unfold_probabilities(
    block: &Block,
    mut sprob_all: ArrayViewMut1<Real>,
    sprob_first: Array1<Real>,
    sprob_recom: Array1<Real>,
    sprob_norecom: Array1<Real>,
) {
    let precomp1 = sprob_recom / block.clustsize.to_owned();
    let precomp2 = sprob_norecom / (sprob_first + *_E);

    Zip::from(&mut sprob_all)
        .and(&block.indmap)
        .apply(|prob, &ui| {
            let ui = ui as usize;
            *prob = precomp1[ui] + *prob * precomp2[ui];
        });
}

// Precompute joint fwd-bwd term for imputation;
// same as "Constants" variable in minimac
fn precompute_joint(
    block: &Block,
    sprob_all: Array1<Real>,
    fwdprob_all: Array1<Real>,
) -> Array1<Real> {
    let precomp = sprob_all * fwdprob_all;

    #[cfg(not(feature = "leak-resistant"))]
    {
        let mut jprob = Array1::<Real>::zeros(block.nuniq);
        Zip::from(&block.indmap).and(&precomp).apply(|&ind, p| {
            jprob[ind as usize] += p;
        });
        jprob
    }

    #[cfg(feature = "leak-resistant")]
    {
        let mut jprob = vec![Vec::with_capacity(20); block.nuniq];
        for (&ind, &p) in block.indmap.iter().zip(precomp.into_iter()) {
            jprob[ind as usize].push(p);
        }
        Array1::from(
            jprob
                .into_iter()
                .map(|mut v| Real::sum_in_place(v.as_mut_slice()))
                .collect::<Vec<Real>>(),
        )
    }
}

#[allow(non_snake_case)]
fn impute(
    jprob: Array1<Real>,
    clustsize: ArrayView1<Real>,
    rhap_row: &BitSlice,
    fwdprob_row: Array1<Real>,
    fwdprob_first: Array1<Real>,
    fwdprob_norecom_row: Array1<Real>,
    sprob: Array1<Real>,
    sprob_first: Array1<Real>,
    sprob_norecom: Array1<Real>,
) -> Real {
    let E = *_E;
    let combined = {
        let x = fwdprob_norecom_row * sprob_norecom;
        jprob * (x.clone() / (fwdprob_first * sprob_first + E))
            + (fwdprob_row * sprob - x) / clustsize
    };

    #[cfg(not(feature = "leak-resistant"))]
    let (p0, p1) = combined
        .iter()
        .zip(rhap_row.iter())
        .fold((0., 0.), |mut acc, (&c, rsym)| {
            if *rsym {
                acc.1 += c;
                acc
            } else {
                acc.0 += c;
                acc
            }
        });

    #[cfg(feature = "leak-resistant")]
    let (p0, p1) = {
        let mut r_iter = rhap_row.iter();
        let (mut p1, mut p0): (Vec<_>, Vec<_>) =
            combined.into_iter().partition(|_| *r_iter.next().unwrap());
        (
            Real::sum_in_place(p0.as_mut_slice()),
            Real::sum_in_place(p1.as_mut_slice()),
        )
    };
    p1 / (p1 + p0)
}
