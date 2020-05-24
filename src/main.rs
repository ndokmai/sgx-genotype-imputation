#![allow(dead_code)]
mod params;
mod symbol;

use ndarray::prelude::*;
use params::Params;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use symbol::Symbol;

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

fn log_fwd_wind(
    mg: &[Symbol],
    params: &Params,
    num_refs: usize,
    win_start: usize,
    win_size: usize,
) -> Array2<f64> {
    let mut s = Array2::zeros((win_size, num_refs));
    let init = &params.init;
    let emit = &params.emit;
    let tran = &params.tran;

    let k = mg[win_start].pos();

    s.slice_mut(s![0, ..])
        .assign(&(init.map(|x| x.ln()) + emit.slice(s![k, .., win_start]).map(|x| x.ln())));

    for i in 1..win_size {
        let k = mg[win_start + i].pos();
        for j in 0..num_refs {
            let temp = &s.slice(s![i - 1, ..])
                + &tran.slice(s![.., j]).map(|x| x.ln())
                + emit[[k, j, win_start + i]].ln();
            s[[i, j]] = lse_ndarray(temp.view());
        }
    }

    return s;
}

fn log_bwd_wind(
    mg: &[Symbol],
    params: &Params,
    num_refs: usize,
    win_start: usize,
    win_size: usize,
) -> Array2<f64> {
    let mut r = Array2::zeros((win_size, num_refs));
    let tran = &params.tran;
    let emit = &params.emit;

    r.slice_mut(s![win_size - 1, ..]).fill(0.);

    for i in (0..win_size - 1).rev() {
        let k = mg[win_start].pos();
        for j in (0..num_refs).rev() {
            let temp = &r.slice(s![i + 1, ..])
                + &tran.slice(s![j, ..]).map(|x| x.ln())
                + &emit.slice(s![k, .., win_start + i + 1]).map(|x| x.ln());
            r[[i, j]] = lse_ndarray(temp.view());
        }
    }

    return r;
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: {}\t<Reference_Haplotypes>\t<Imputation_Samples>",
            args[0]
        );
        return;
    }

    let ref_path = env::args().nth(1).unwrap();
    let ref_file =
        BufReader::new(File::open(&ref_path).expect("Cannot open reference haplotypes file"));

    let sample_path = env::args().nth(2).unwrap();
    let sample_file =
        BufReader::new(File::open(&sample_path).expect("Cannot open imputation samples file"));

    // It's safer to parse early to make sure the inputs aren't malformed
    let refs = ref_file
        .lines()
        .map(|l| {
            l.unwrap()
                .chars()
                .map(|c| Symbol::parse(&c).unwrap())
                .collect::<Vec<Symbol>>()
        })
        .collect::<Vec<_>>();

    // Ko: I'm not sure what's happening here. Do you mean to take the last line only?
    let mut mg = Vec::new();
    for line in sample_file.lines() {
        let v_line = line.unwrap();
        // It's safer to parse early to make sure the inputs aren't malformed
        mg = v_line.chars().map(|c| Symbol::parse(&c).unwrap()).collect();
    }

    let params = Params::init(&refs[..], mg.len());

    eprintln!("Running forward-backward algorithm ...");
    let win_size = mg.len() / 10;
    for n in 0..(mg.len() / win_size) {
        //println!("{}", n);
        let log_fw = log_fwd_wind(&mg[..], &params, refs.len(), n * win_size, win_size);
        let log_bw = log_bwd_wind(&mg[..], &params, refs.len(), n * win_size, win_size);
        let log_fb = log_fw + log_bw;

        // Compute and print final imputed sequence
        for i in 0..win_size {
            let mut max_val = f64::NEG_INFINITY;
            let mut max_idx = 0;
            for k in 0..4 {
                let ans = &log_fb.slice(s![i, ..])
                    + &params
                        .emit
                        .slice(s![k, .., (n * win_size) + i])
                        .map(|x| x.ln());
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
