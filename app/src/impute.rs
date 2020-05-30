use crate::params::Params;
use crate::symbol::Symbol;
use ndarray::{s, Array2, ArrayView1};

fn lse_ndarray(x: ArrayView1<f64>) -> f64 {
    let max = x.fold(f64::NEG_INFINITY, |accu, i| f64::max(accu, *i));
    let sum = x.fold(0., |accu, i| accu + (i - max).exp());
    return max + sum.ln();
}

//fn log_fwd(mg: &String, init: &Array<f64, Ix1>, emit: &Array<f64, Ix3>, tran: &Array<f64, Ix2>, n: usize, m: usize) -> Array<f64, Ix2>
//{
//let mut s: Array<f64, _> = Array::zeros((n, m));
//let mut temp: Array<f64, Ix1> = Array::zeros(m);

//let c = mg.chars().nth(0).unwrap();
//let k = match c
//{
//'A' => 0,
//'C' => 1,
//'G' => 2,
//'T' => 3,
//'.' => 4,
//_ => 5,
//};

//for i in 0..m
//{
//s[[0, i]] = init[i].ln() + emit[[k, i, 0]].ln();
//}

//for i in 1..n
//{
//let c = mg.chars().nth(i).unwrap();
//let k = match c
//{
//'A' => 0,
//'C' => 1,
//'G' => 2,
//'T' => 3,
//'.' => 4,
//_ => 5,
//};

//for j in 0..m
//{
//for j2 in 0..m
//{
//temp[j2] = s[[i - 1, j2]] + tran[[j2, j]].ln() + emit[[k, j, i]].ln();
//}
//s[[i, j]] = lse_ndarray(temp.view());
//}
//}

//return s;
//}

//fn log_bwd(mg: &String, emit: &Array<f64, Ix3>, tran: &Array<f64, Ix2>, n: usize, m: usize) -> Array<f64, Ix2>
//{
//let mut r: Array<f64, _> = Array::zeros((n, m));
//let mut temp: Array<f64, Ix1> = Array::zeros(m);

//for i in 0..m
//{
//r[[n - 1, i]] = 0.0;
//}

//for i in (0..n-1).rev()
//{
//let c = mg.chars().nth(i+1).unwrap();
//let k = match c
//{
//'A' => 0,
//'C' => 1,
//'G' => 2,
//'T' => 3,
//'.' => 4,
//_ => 5,
//};

//for j in (0..m).rev()
//{
//for j2 in (0..m).rev()
//{
//temp[j2] = r[[i + 1, j2]] + tran[[j, j2]].ln() + emit[[k, j2, i + 1]].ln();
//}
//r[[i, j]] = lse_ndarray(temp.view());
//}
//}

//return r;
//}
//

//fn log_fwd(input_window: ArrayView1<Symbol>, params: &Params) -> Array2<f64> {
//let (init, emit, tran) = params.get_views();
//let nrefs = params.nrefs;
//let mut s = unsafe { Array2::<f64>::uninitialized((input_window.len(), params.nrefs)); };
//let k = input_window.first().unwrap().pos();
//s.slice_mut(s![0, ..])
//.assign(&(init.map(|x| x.ln()) + emit.slice(s![k, .., win_start]).map(|x| x.ln())));

//unimplemented!()
//}

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
        .assign(&(&init + &emit.slice(s![k, .., win_start])));

    for i in 1..win_size {
        let k = input[[win_start + i]].pos();
        for j in 0..nrefs {
            let temp =
                &s.slice(s![i - 1, ..]) + &tran.slice(s![.., j]) + emit[[k, j, win_start + i]];
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
                + &emit.slice(s![k, .., win_start + i + 1]);
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
                    &log_fb.slice(s![i, ..]) + &params.emit.slice(s![k, .., (n * win_size) + i]);
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
