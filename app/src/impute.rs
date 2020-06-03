use crate::params::Params;
use crate::symbol::Symbol;
use ndarray::{s, Array2, ArrayView1};

fn lse_ndarray(x: ArrayView1<f64>) -> f64 {
    let max = x.fold(f64::NEG_INFINITY, |accu, i| f64::max(accu, *i));
    let sum = x.fold(0., |accu, i| accu + (i - max).exp());
    return max + sum.ln();
}

fn log_fwd_window_single(
    input: ArrayView1<Symbol>,
    win_start: usize,
    win_size: usize,
    params: &Params,
) -> Array2<f64> {
    let nrefs = params.nrefs;
    let mut s = unsafe { Array2::<f64>::uninitialized((win_size, nrefs)) };
    let (init, emit, tran) = params.get_views();

    let k = input[[win_start]].pos();

    s.slice_mut(s![0, ..])
        .assign(&(&init + &emit.slice(s![win_start, k, ..,])));

    for i in 1..win_size {
        let k = input[[win_start + i]].pos();
        for j in 0..nrefs {
            let temp =
                &s.slice(s![i - 1, ..]) + &tran.slice(s![.., j]) + emit[[win_start + i, k, j]];
            s[[i, j]] = lse_ndarray(temp.view());
        }
    }

    return s;
}

fn log_bwd_window_single(
    input: ArrayView1<Symbol>,
    win_start: usize,
    win_size: usize,
    params: &Params,
) -> Array2<f64> {
    let nrefs = params.nrefs;
    let mut r = unsafe { Array2::<f64>::uninitialized((win_size, nrefs)) };
    let (_, emit, tran) = params.get_views();

    r.slice_mut(s![win_size - 1, ..]).fill(0.);

    for i in (0..win_size - 1).rev() {
        let k = input[[win_start]].pos();
        for j in (0..nrefs).rev() {
            let temp = &r.slice(s![i + 1, ..])
                + &tran.slice(s![j, ..])
                + &emit.slice(s![win_start + i + 1, k, ..]);
            r[[i, j]] = lse_ndarray(temp.view());
        }
    }

    return r;
}

/// Impute multiple inputs
pub fn impute_single(inputs: ArrayView1<Symbol>, params: &Params) {
    let win_size = inputs.len() / 10;
    for n in 0..(inputs.len() / win_size) {
        //println!("{}", n);
        let log_fw = log_fwd_window_single(inputs, n * win_size, win_size, params);
        let log_bw = log_bwd_window_single(inputs, n * win_size, win_size, params);
        let log_fb = log_fw + log_bw;

        // Compute and print final imputed sequence
        for i in 0..win_size {
            let mut max_val = f64::NEG_INFINITY;
            let mut max_idx = 0;
            for k in 0..4 {
                let ans =
                    &log_fb.slice(s![i, ..]) + &params.emit.slice(s![(n * win_size) + i, k, ..]);
                let ans_max = ans
                    .iter()
                    .fold(f64::NEG_INFINITY, |accu, i| f64::max(accu, *i));
                if ans_max > max_val {
                    max_val = ans_max;
                    max_idx = k;
                }
            }
            eprint!("{}", Symbol::from_pos(max_idx).unwrap());
        }
        eprintln!();
    }
}
