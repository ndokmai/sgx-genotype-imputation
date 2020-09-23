use crate::block::Block;
use crate::cache::*;
use crate::output::OutputWrite;
use crate::ref_panel::RefPanelRead;
use crate::symbol::Symbol;
use crate::symbol_vec::SymbolVec;
use crate::{Input, Real};
use bitvec::prelude::{BitSlice, BitVec, Lsb0};
use lazy_static::lazy_static;
use ndarray::{s, Array1, ArrayView1, ArrayViewMut1, Zip};
use std::convert::TryFrom;

#[cfg(feature = "leak-resistant")]
mod leak_resistant_mod {
    pub use crate::bacc::Bacc;
    pub use timing_shield::{TpEq, TpI8, TpOrd};
}

#[cfg(feature = "leak-resistant")]
use leak_resistant_mod::*;

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

pub fn impute_all(
    mut thap_ind: impl Iterator<Item = bool>,
    mut thap_dat: impl Iterator<Item = Input>,
    mut ref_panel: impl RefPanelRead,
    mut cache: impl Cache,
    mut output_writer: impl OutputWrite<Real>,
) {
    let m = ref_panel.n_haps();
    let m_real: Real = u16::try_from(m).unwrap().into();
    let n_blocks = ref_panel.n_blocks();

    let mut block_cache = cache.new_save();
    let mut thap_block_cache = cache.new_save();
    let mut fwdprob_cache = cache.new_save();
    let mut fwdprob_norecom_cache = cache.new_save();
    let mut fwdprob_first_cache = cache.new_save();
    let mut fwdprob_all_cache = cache.new_save();

    let mut sprob_all = Array1::<Real>::ones(m); // unnormalized

    // Forward pass
    for b in 0..n_blocks {
        let mut thap_dat_block = SymbolVec::new();
        let mut thap_ind_block = BitVec::<Lsb0, u64>::new();
        let block = if b == 0 {
            // First position emission (edge case)
            let first_block = ref_panel.next().unwrap();
            let first_ind = thap_ind.next().unwrap();
            thap_ind_block.push(first_ind);
            if first_ind {
                let first_dat = thap_dat.next().unwrap();

                #[cfg(not(feature = "leak-resistant"))]
                thap_dat_block.push(first_dat);

                #[cfg(feature = "leak-resistant")]
                thap_dat_block.push(first_dat.expose().into());

                first_emission(first_dat, &first_block, sprob_all.view_mut());
            }
            first_block
        } else {
            ref_panel.next().unwrap()
        };

        fwdprob_all_cache.push(sprob_all.clone());

        let mut fwdprob = Vec::new();
        let mut fwdprob_norecom = Vec::new();

        let mut sprob = fold_probabilities(sprob_all.view(), &block);

        let sprob_first = sprob.clone();
        let mut sprob_norecom = sprob.clone();

        // Cache forward probabilities at first position
        fwdprob.push(sprob.clone());
        fwdprob_norecom.push(sprob_norecom.clone());

        // Walk
        Zip::from(block.rprob.slice(s![..block.nvar - 1]))
            .and(block.afreq.slice(s![1..]))
            .and(&block.rhap[1..])
            .apply(|&rec, &block_afreq, rhap_row| {
                let tflag = thap_ind.next().unwrap();
                thap_ind_block.push(tflag);
                transition(
                    rec,
                    m_real,
                    block.clustsize.view(),
                    sprob.view_mut(),
                    sprob_norecom.view_mut(),
                );

                if tflag {
                    let tsym = thap_dat.next().unwrap();

                    #[cfg(not(feature = "leak-resistant"))]
                    thap_dat_block.push(tsym);

                    #[cfg(feature = "leak-resistant")]
                    thap_dat_block.push(tsym.expose().into());

                    later_emission(
                        tsym,
                        sprob.view_mut(),
                        sprob_norecom.view_mut(),
                        block_afreq,
                        rhap_row,
                    );
                }

                fwdprob.push(sprob.clone());
                fwdprob_norecom.push(sprob_norecom.clone());
            });

        let sprob_recom = &sprob - &sprob_norecom;

        // Skip last block
        if b < n_blocks - 1 {
            unfold_probabilities(
                &block,
                sprob_all.view_mut(),
                sprob_first.view(),
                sprob_recom.view(),
                sprob_norecom.view(),
            );
        }
        thap_ind_block.shrink_to_fit();
        thap_dat_block.shrink_to_fit();
        thap_block_cache.push((thap_ind_block, thap_dat_block));
        block_cache.push(block);
        fwdprob_cache.push(fwdprob);
        fwdprob_norecom_cache.push(fwdprob_norecom);
        fwdprob_first_cache.push(sprob_first);
    }

    drop(thap_ind);
    drop(thap_dat);
    drop(ref_panel);

    let mut block_cache = block_cache.into_load();
    let mut thap_block_cache = thap_block_cache.into_load();
    let mut fwdprob_cache = fwdprob_cache.into_load();
    let mut fwdprob_norecom_cache = fwdprob_norecom_cache.into_load();
    let mut fwdprob_first_cache = fwdprob_first_cache.into_load();
    let mut fwdprob_all_cache = fwdprob_all_cache.into_load();

    // Backward pass
    let mut sprob_all = Array1::<Real>::ones(m);
    for b in (0..n_blocks).rev() {
        let block = block_cache.pop().unwrap();
        let (mut thap_ind_block, mut thap_dat_block) = thap_block_cache.pop().unwrap();
        let fwdprob = fwdprob_cache.pop().unwrap();
        let fwdprob_norecom = fwdprob_norecom_cache.pop().unwrap();
        let fwdprob_first = fwdprob_first_cache.pop().unwrap();
        let fwdprob_all = fwdprob_all_cache.pop().unwrap();

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
            output_writer.push(impute(
                jprob.view(),
                block.clustsize.view(),
                block.rhap[j].as_bitslice(),
                fwdprob[j].view(),
                fwdprob_first.view(),
                fwdprob_norecom[j].view(),
                sprob.view(),
                sprob_first.view(),
                sprob_norecom.view(),
            ));

            if thap_ind_block.pop().unwrap() {
                #[cfg(not(feature = "leak-resistant"))]
                let tsym = thap_dat_block.pop().unwrap();

                #[cfg(feature = "leak-resistant")]
                let tsym = Input::protect(thap_dat_block.pop().unwrap().into());

                later_emission(
                    tsym,
                    sprob.view_mut(),
                    sprob_norecom.view_mut(),
                    block.afreq[j],
                    block.rhap[j].as_bitslice(),
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
            if b == 0 && j == 1 {
                output_writer.push(impute(
                    jprob.view(),
                    block.clustsize.view(),
                    block.rhap[0].as_bitslice(),
                    fwdprob[0].view(),
                    fwdprob_first.view(),
                    fwdprob_norecom[0].view(),
                    sprob.view(),
                    sprob_first.view(),
                    sprob_norecom.view(),
                ));
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
    }
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

fn single_emission(tsym: Input, block_afreq: f32, rhap: Symbol) -> Real {
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

fn first_emission(tsym: Input, block: &Block, mut sprob_all: ArrayViewMut1<Real>) {
    let afreq = block.afreq[0];

    #[cfg(not(feature = "leak-resistant"))]
    if tsym != Symbol::Missing {
        Zip::from(&mut sprob_all).and(&block.indmap).apply(|p, &i| {
            let emi = single_emission(tsym, afreq, block.rhap[0][i as usize].into());
            *p *= emi
        });
    }

    #[cfg(feature = "leak-resistant")]
    {
        let cond = tsym.tp_not_eq(&-1);
        Zip::from(&mut sprob_all).and(&block.indmap).apply(|p, &i| {
            let emi = single_emission(tsym, afreq, block.rhap[0][i as usize].into());
            *p = cond.select(emi, *p);
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
    if tsym != Symbol::Missing {
        sprob
            .iter_mut()
            .zip(sprob_norecom.iter_mut())
            .zip(rhap_row.iter())
            .for_each(|((p, p_norecom), &rhap)| {
                let emi = single_emission(tsym, block_afreq, rhap.into());
                *p *= emi;
                *p_norecom *= emi;
            });
    }
    #[cfg(feature = "leak-resistant")]
    {
        let cond = tsym.tp_not_eq(&-1);
        sprob
            .iter_mut()
            .zip(sprob_norecom.iter_mut())
            .zip(rhap_row.iter())
            .for_each(|((p, p_norecom), &rhap)| {
                let emi = single_emission(tsym, block_afreq, rhap.into());
                *p = cond.select(*p * emi, *p);
                *p_norecom = cond.select(*p_norecom * emi, *p_norecom);
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
