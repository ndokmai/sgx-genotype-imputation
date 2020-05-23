extern crate ndarray;

use ndarray::prelude::*;

use std::env;
use std::fs::File;
use std::io::{BufReader, BufRead};

fn lse_ndarray(x: &Array<f64, Ix1>) -> f64
{
	let mut max: f64 = std::f64::MIN;

	for i in 0..x.len()
	{
		if x[i] > max
		{
			max = x[i];
		}
	}

	let mut sum: f64 = 0.0;

	for i in 0..x.len()
	{
		sum = sum + (x[i] - max).exp();
	}

	return max + sum.ln();
}

fn log_fwd(mg: &String, init: &Array<f64, Ix1>, emit: &Array<f64, Ix3>, tran: &Array<f64, Ix2>, n: usize, m: usize) -> Array<f64, Ix2>
{
	let mut s: Array<f64, _> = Array::zeros((n, m));
	let mut temp: Array<f64, Ix1> = Array::zeros(m);

	let c = mg.chars().nth(0).unwrap();
	let k = match c
	{
		'A' => 0,
			'C' => 1,
			'G' => 2,
			'T' => 3,
			'.' => 4,
			_ => 5,
	};

	for i in 0..m
	{
		s[[0, i]] = init[i].ln() + emit[[k, i, 0]].ln();
	}

	for i in 1..n
	{
		let c = mg.chars().nth(i).unwrap();
		let k = match c
		{
			'A' => 0,
				'C' => 1,
				'G' => 2,
				'T' => 3,
				'.' => 4,
				_ => 5,
		};

		for j in 0..m
		{
			for j2 in 0..m
			{
				temp[j2] = s[[i - 1, j2]] + tran[[j2, j]].ln() + emit[[k, j, i]].ln();
			}
			s[[i, j]] = lse_ndarray(&temp);
		}
	}

	return s;
}

fn log_bwd(mg: &String, emit: &Array<f64, Ix3>, tran: &Array<f64, Ix2>, n: usize, m: usize) -> Array<f64, Ix2>
{
	let mut r: Array<f64, _> = Array::zeros((n, m));
	let mut temp: Array<f64, Ix1> = Array::zeros(m);

	for i in 0..m
	{
		r[[n - 1, i]] = 0.0;
	}

	for i in (0..n-1).rev()
	{
		let c = mg.chars().nth(i+1).unwrap();
		let k = match c
		{
			'A' => 0,
				'C' => 1,
				'G' => 2,
				'T' => 3,
				'.' => 4,
				_ => 5,
		};

		for j in (0..m).rev()
		{
			for j2 in (0..m).rev()
			{
				temp[j2] = r[[i + 1, j2]] + tran[[j, j2]].ln() + emit[[k, j2, i + 1]].ln();
			}
			r[[i, j]] = lse_ndarray(&temp);
		}
	}

	return r;
}

fn log_fwd_wind(mg: &String, init: &Array<f64, Ix1>, emit: &Array<f64, Ix3>, tran: &Array<f64, Ix2>, num_refs: usize, win_start: usize, win_size: usize) -> Array<f64, Ix2>
{
	let mut s: Array<f64, _> = Array::zeros((win_size, num_refs));
	let mut temp: Array<f64, Ix1> = Array::zeros(num_refs);

	let c = mg.chars().nth(win_start).unwrap();
	let k = match c
	{
		'A' => 0,
		'C' => 1,
		'G' => 2,
		'T' => 3,
		'.' => 4,
		_ => 5,
	};

	for i in 0..num_refs
	{
		s[[0, i]] = init[i].ln() + emit[[k, i, win_start]].ln();
	}

	for i in 1..win_size
	{
		let c = mg.chars().nth(win_start + i).unwrap();
		let k = match c
		{
			'A' => 0,
			'C' => 1,
			'G' => 2,
			'T' => 3,
			'.' => 4,
			_ => 5,
		};

		for j in 0..num_refs
		{
			for j2 in 0..num_refs
			{
				temp[j2] = s[[i - 1, j2]] + tran[[j2, j]].ln() + emit[[k, j, win_start + i]].ln();
			}
			s[[i, j]] = lse_ndarray(&temp);
		}
	}

	return s;
}

fn log_bwd_wind(mg: &String, emit: &Array<f64, Ix3>, tran: &Array<f64, Ix2>, num_refs: usize, win_start: usize, win_size: usize) -> Array<f64, Ix2>
{
	let mut r: Array<f64, _> = Array::zeros((win_size, num_refs));
	let mut temp: Array<f64, Ix1> = Array::zeros(num_refs);

	for i in 0..num_refs
	{
		r[[win_size - 1, i]] = 0.0;
	}

	for i in (0..win_size-1).rev()
	{
		let c = mg.chars().nth(win_start + i + 1).unwrap();
		let k = match c
		{
			'A' => 0,
			'C' => 1,
			'G' => 2,
			'T' => 3,
			'.' => 4,
			_ => 5,
		};

		for j in (0..num_refs).rev()
		{
			for j2 in (0..num_refs).rev()
			{
				temp[j2] = r[[i + 1, j2]] + tran[[j, j2]].ln() + emit[[k, j2, win_start + i + 1]].ln();
			}
			r[[i, j]] = lse_ndarray(&temp);
		}
	}

	return r;
}

fn main()
{
	let args: Vec<_> = env::args().collect();
	if args.len() != 3
	{
		println!("Usage: {}\t<Reference_Haplotypes>\t<Imputation_Samples>", args[0]);
		return;
	}

	let ref_path = env::args().nth(1).unwrap();
	let ref_file = BufReader::new(File::open(&ref_path).unwrap());

	let sample_path = env::args().nth(2).unwrap();
	let sample_file = BufReader::new(File::open(&sample_path).unwrap());

	let mut refs = Vec::new();
	for line in ref_file.lines()
	{
		let v_line = line.unwrap();
		let ref_seq = String::from(v_line);
		refs.push(ref_seq);
	}

	let mut mg = String::new();
	for line in sample_file.lines()
	{
		let v_line = line.unwrap();
		mg = v_line;
	}

	let num_refs: usize = refs.len();
	let mut init: Array<f64, _> = Array::zeros(num_refs);
	let mut emit: Array<f64, _> = Array::zeros((5, num_refs, mg.len()));
	let mut tran: Array<f64, _> = Array::zeros((num_refs, num_refs));

	let symbols = array!['A', 'C', 'G', 'T', '.'];

	println!("Setting initial probabilities ...");
	for i in 0..num_refs
	{
		init[i] = 1.0 / (num_refs as f64);
	}

	println!("Setting transition probabilities ...");
	for i in 0..num_refs
	{
		for j in 0..num_refs
		{
			if i == j
			{
				tran[[i, j]] = 0.60;
			}
			else
			{
				tran[[i, j]] = (1.00 - 0.60) / (num_refs as f64);
			}
		}
	}

	println!("Setting emission probabilities ...");
	for i in 0..5
	{
		for j in 0..num_refs
		{
			for k in 0..mg.len()
			{
				if i == 4
				{
					emit[[i, j, k]] = 1.0;
				}
				else if refs[j].chars().nth(k).unwrap() == symbols[i]
				{
					emit[[i, j, k]] = 0.60;
				}
				else
				{
					emit[[i, j, k]] = 0.40;
				}
			}
		}
	}

	println!("Running forward-backward algorithm ...");
	let win_size = mg.len() / 10;
	for n in 0..(mg.len() / win_size)
	{
		//println!("{}", n);
		let log_fw = log_fwd_wind(&mg.to_ascii_uppercase(), &init, &emit, &tran, refs.len(), n * win_size, win_size);
		let log_bw = log_bwd_wind(&mg.to_ascii_uppercase(), &emit, &tran, refs.len(), n * win_size, win_size);
		let log_fb = log_fw + log_bw;


		// Compute and print final imputed sequence
		for i in 0..win_size
		{
			let mut max_val = std::f64::MIN;
			let mut max_idx = 0;
			for k in 0..4
			{
				for j in 0..refs.len()
				{
					let ans = log_fb[[i, j]] + emit[[k, j, (n * win_size) + i]].ln();
					if ans > max_val
					{
						max_val = ans;
						max_idx = k;
					}
				}
			}
			print!("{}", symbols[max_idx]);
		}
		println!();
	}
}
