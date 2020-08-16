use ndarray::Array2;
use std::fs::File;
use std::io::{BufReader, BufRead};
use regex::Regex;
use std::io::prelude::*;

static REF_FILE: &'static str = "largeref.m3vcf";
static INPUT_FILE: &'static str = "input.txt";
static OUTPUT_FILE: &'static str = "output.txt";

struct Block {
    indmap: Vec<usize>,
    nvar: usize,
    nuniq: usize,
    clustsize: Vec<usize>,
    rhap: Array2<i8>,
    eprob: Vec<f64>,
    rprob: Vec<f64>,
    afreq: Vec<f64>,
}

// chunk_id represents predefined mapping from
// integers to large genomic windows which are
// imputed independently
fn load_chunk_from_input(chunk_id: usize) -> Vec<i8> {
    // TODO: chunk_id is currently ignored
    // and the entire toy data is loaded
    println!("Loading chunk {} from input ({})", chunk_id, INPUT_FILE);

    let n = 97020; // TODO: hardcoded variant count
    let mut x = vec![0; n];

    let f = File::open(INPUT_FILE).expect("Unable to open input file");
    let f = BufReader::new(f);

    let mut ind: usize = 0;
    for line in f.lines() {
        let line = line.expect("Unable to read line from input file");
        match line.parse::<i8>() {
            Ok(n) => {
                x[ind] = n;
                ind += 1;
            },
            Err(_) => panic!("Parsing error in input file"),
        }
    }

    x
}

fn create_block(indcnt: usize, nvar: usize, nuniq: usize) -> Block {
    Block {
        indmap: vec![0; indcnt],
        nvar: nvar,
        nuniq: nuniq,
        clustsize: vec![0; nuniq],
        rhap: Array2::<i8>::zeros((nvar, nuniq)),
        eprob: vec![0.0; nvar],
        rprob: vec![0.0; nvar],
        afreq: vec![0.0; nvar],
    }
}

// chunk_id represents predefined mapping from
// integers to large genomic windows which are
// imputed independently
fn load_chunk_from_refpanel(chunk_id: usize) -> Vec<Block>  {
    // TODO: chunk_id is currently ignored
    // and the entire toy data is loaded
    println!("Loading chunk {} from reference panel ({})", chunk_id, REF_FILE);

    let m = 5008; // TODO: hardcoded reference panel count

    let f = File::open(REF_FILE).expect("Unable to open reference file");
    let f = BufReader::new(f);

    let mut blocks = vec![];

    let var_re = Regex::new(r"VARIANTS=(\d+)").unwrap();
    let hap_re = Regex::new(r"REPS=(\d+)").unwrap();
    let err_re = Regex::new(r"Err=(\d*\.?\d+)").unwrap();
    let rec_re = Regex::new(r"Recom=(\d*\.?\d+)").unwrap();

    let mut lines_left: usize = 0;
    let mut cur_block: usize = 0;
    let mut cur_var: usize = 0;
    for line in f.lines() {
        let line = line.expect("Unable to read line from reference file");
        if &line[0..1] == "#" {
            continue;
        }

        if lines_left == 0 { // block header

            let mut iter = line.split_ascii_whitespace();

            let tok = iter.nth(7).unwrap(); // info field
            iter.next(); // skip one column

            let nvar = var_re.captures_iter(tok).next().unwrap()[1]
                .parse().expect("parse error");
            let nhap = hap_re.captures_iter(tok).next().unwrap()[1]
                .parse().expect("parse error");

            blocks.push(create_block(m, nvar, nhap));

            blocks[cur_block].indmap =
                iter.map(|s| s.parse().expect("parse error")).collect();

            for i in 0..blocks[cur_block].indmap.len() {
                let v: usize = blocks[cur_block].indmap[i];
                blocks[cur_block].clustsize[v] += 1;
            }

            lines_left += nvar;

        } else { // individual variants in a block

            let mut iter = line.split_ascii_whitespace();

            let tok = iter.nth(7).unwrap(); // info field
            let eprob = err_re.captures_iter(tok).next().unwrap()[1]
                .parse().expect("parse error");
            let rprob = rec_re.captures_iter(tok).next().unwrap()[1]
                .parse().expect("parse error");
            let data = iter.next().unwrap(); // data for one variant

            blocks[cur_block].eprob[cur_var] = eprob;
            blocks[cur_block].rprob[cur_var] = rprob;

            let mut ind = 0;
            let mut alt_count = 0;
            for b in data.chars() { // TODO: Make this more efficient
                let geno = b.to_digit(10).unwrap() as i8; 
                blocks[cur_block].rhap[[cur_var,ind]] = geno;
                if geno == 1 { // allele freq
                    alt_count += blocks[cur_block].clustsize[ind];
                }
                ind += 1;
            }

            blocks[cur_block].afreq[cur_var] = (alt_count as f64) / (m as f64);

            cur_var += 1;
            lines_left -= 1;

            if lines_left == 0 {
                cur_block += 1;
                cur_var = 0;
            }
        }

    }

    blocks
}

fn impute_chunk(chunk_id: usize) -> Vec<f64> {

    let m = 5008; // TODO: hardcoded reference panel count

    let thap = load_chunk_from_input(chunk_id);
    let blocks = load_chunk_from_refpanel(chunk_id);
    let mut imputed = vec![0.0; thap.len()];

    let mut fwdcache = Vec::new();
    let mut fwdcache_norecom = Vec::new();
    let mut fwdcache_first = Vec::new();
    let mut fwdcache_all = Array2::<f64>::zeros((blocks.len(), m));
    
    //println!("Number of blocks: {}", blocks.len());

    /* Forward pass */
    let mut sprob_all = vec![1.0; m]; // unnormalized
    let mut var_offset: usize = 0;

    // First position emission (edge case)
    if thap[0] != -1 {
        let block = &blocks[0];
        let err = 0.00999;
        let tsym = thap[0];
        let afreq = if tsym == 1 {
            block.afreq[0]
        } else {
            1.0 - block.afreq[0]
        };
        let background = 1e-5;

        for i in 0..m {
            let emi = if tsym == block.rhap[[0,block.indmap[i]]] {
                (1.0 - err) + err * afreq + background
            } else {
                err * afreq + background
            };

            sprob_all[i] *= emi;
        }
    }

    for b in 0..blocks.len() {
        let block = &blocks[b];

        let mut fwdprob = Array2::<f64>::zeros((block.nvar, block.nuniq));
        let mut fwdprob_norecom = Array2::<f64>::zeros((block.nvar, block.nuniq));

        // Fold probabilities
        let mut sprob = vec![0.0; block.nuniq];
        for i in 0..m {
            sprob[block.indmap[i]] += sprob_all[i];
            fwdcache_all[[b,i]] = sprob_all[i]; // save cache
        }

        let sprob_first = sprob.to_vec();
        let mut sprob_norecom = sprob.to_vec();

        // Walk
        for j in 1..block.nvar {
            let rec = block.rprob[j-1];
            // TODO: for some reason minimac ignores error prob in input m3vcf
            //       and always uses 0.00999 as below. need to investigate further
            //let err = block.eprob[j];
            let err = 0.00999;
            let tsym = thap[var_offset + j];
            let afreq = if tsym == 1 {
                block.afreq[j]
            } else {
                1.0 - block.afreq[j]
            };
            let background = 1e-5;

            // Transition
            let mut sprob_tot = 0.0;
            for i in 0..block.nuniq {
                sprob_tot += sprob[i];
                sprob_norecom[i] *= 1.0 - rec;
            }

            sprob_tot *= rec / (m as f64);

            let mut complement = 1.0 - rec;
            // Lazy normalization (same as minimac)
            if sprob_tot < 1e-20 { 
                let scale_factor = 1e10;
                sprob_tot *= scale_factor;
                complement *= scale_factor;
                for i in 0..block.nuniq {
                    sprob_norecom[i] *= scale_factor;
                }
            }

            for i in 0..block.nuniq {
                sprob[i] = complement * sprob[i]
                    + (block.clustsize[i] as f64) * sprob_tot;
            }

            // Emission
            if tsym != -1 {
                for i in 0..block.nuniq {

                    let emi = if tsym == block.rhap[[j,i]] {
                        (1.0 - err) + err * afreq + background
                    } else {
                        err * afreq + background
                    };

                    sprob[i] *= emi;
                    sprob_norecom[i] *= emi;
                }
            }

            // Cache forward probabilities
            for i in 0..block.nuniq {
                fwdprob[[j,i]] = sprob[i];
                fwdprob_norecom[[j,i]] = sprob_norecom[i];
            }
        }

        fwdcache.push(fwdprob);
        fwdcache_norecom.push(fwdprob_norecom);
        fwdcache_first.push(vec![0.0; block.nuniq]);
        for i in 0..block.nuniq {
            fwdcache_first[b][i] = sprob_first[i];
        }


        let mut sprob_recom = vec![0.0; block.nuniq];
        for i in 0..block.nuniq {
            sprob_recom[i] = (sprob[i] - sprob_norecom[i]).max(0.0);
        }

        // Unfold probabilities
        if b < blocks.len()-1 { // Skip last block
            for i in 0..m {
                let ui = block.indmap[i];
                // TODO: precompute ui terms outside of this for loop
                sprob_all[i] = (sprob_recom[ui] / (block.clustsize[ui] as f64))
                    + (sprob_all[i] * (sprob_norecom[ui] / (sprob_first[ui] + 1e-30)));
            }
        }

        var_offset += block.nvar - 1;
    }

    /* Backward pass */ 
    // TODO: refactor to remove redundancy with forward pass 
    let mut sprob_all = vec![1.0; m];
    let mut var_offset: usize = 0;
    for b in (0..blocks.len()).rev() {
        let block = &blocks[b];
        let fwdprob = &fwdcache[b];
        let fwdprob_norecom = &fwdcache_norecom[b];
        let fwdprob_first = &fwdcache_first[b];

        // Precompute joint fwd-bwd term for imputation;
        // same as "Constants" variable in minimac
        let mut jprob = vec![0.0; block.nuniq];
        for i in 0..m {
            jprob[block.indmap[i]] += fwdcache_all[[b,i]] * sprob_all[i];
        }

        // Fold probabilities
        let mut sprob = vec![0.0; block.nuniq];
        for i in 0..m {
            sprob[block.indmap[i]] += sprob_all[i];
        }

        let sprob_first = sprob.to_vec();
        let mut sprob_norecom = sprob.to_vec();
        
        // Walk
        for j in (1..block.nvar).rev() {
            let rec = block.rprob[j-1];
            // TODO: for some reason minimac ignores error prob in input m3vcf
            //       and always uses 0.00999 as below. need to investigate further
            //let err = block.eprob[j];
            let err = 0.00999;
            let varind = thap.len() - (var_offset + (block.nvar - j));
            let tsym = thap[varind];
            let afreq = if tsym == 1 {
                block.afreq[j]
            } else {
                1.0 - block.afreq[j]
            };
            let background = 1e-5;

            // Impute
            let mut p1 = 0.0;
            let mut p0 = 0.0;
            for i in 0..block.nuniq {

                let combined = jprob[i] * (fwdprob_norecom[[j,i]] * sprob_norecom[i]
                    / (fwdprob_first[i] * sprob_first[i] + 1e-30))
                    + (fwdprob[[j,i]] * sprob[i] - fwdprob_norecom[[j,i]] * sprob_norecom[i])
                    / (block.clustsize[i] as f64);

                let rsym = block.rhap[[j,i]];
                if rsym == 1 {
                    p1 += combined;
                } else if rsym == 0 {
                    p0 += combined;
                }
            }
            imputed[varind] = p1 / (p1 + p0);

            // Emission
            if tsym > -1 { // not missing
                for i in 0..block.nuniq {
                    
                    let emi = if tsym == block.rhap[[j,i]] {
                        (1.0 - err) + err * afreq + background
                    } else {
                        err * afreq + background
                    };

                    sprob[i] *= emi;
                    sprob_norecom[i] *= emi;
                }
            }

            // Transition
            let mut sprob_tot = 0.0;
            for i in 0..block.nuniq {
                sprob_tot += sprob[i];
                sprob_norecom[i] *= 1.0 - rec;
            }

            sprob_tot *= rec / (m as f64);

            let mut complement = 1.0 - rec;
            // Lazy normalization (same as minimac)
            if sprob_tot < 1e-20 { 
                let scale_factor = 1e10;
                sprob_tot *= scale_factor;
                complement *= scale_factor;
                for i in 0..block.nuniq {
                    sprob_norecom[i] *= scale_factor;
                }
            }

            for i in 0..block.nuniq {
                sprob[i] = complement * sprob[i]
                    + (block.clustsize[i] as f64) * sprob_tot;
            }

            // Impute very first position (edge case)
            // TODO: refactor with the same code block above
            if b == 0 && j == 1 {
                let mut p1 = 0.0;
                let mut p0 = 0.0;
                for i in 0..block.nuniq {

                    let combined = jprob[i] * (fwdprob_norecom[[0,i]] * sprob_norecom[i]
                        / (fwdprob_first[i] * sprob_first[i] + 1e-30))
                        + (fwdprob[[0,i]] * sprob[i] - fwdprob_norecom[[0,i]] * sprob_norecom[i])
                        / (block.clustsize[i] as f64);

                    let rsym = block.rhap[[0,i]];
                    if rsym == 1 {
                        p1 += combined;
                    } else if rsym == 0 {
                        p0 += combined;
                    }
                }
                imputed[0] = p1 / (p1 + p0);
            }

        }

        let mut sprob_recom = vec![0.0; block.nuniq];
        for i in 0..block.nuniq {
            sprob_recom[i] = (sprob[i] - sprob_norecom[i]).max(0.0);
        }

        // Unfold probabilities
        if b > 0 {
            for i in 0..m {
                let ui = block.indmap[i];
                // TODO: precompute ui terms outside of this for loop
                sprob_all[i] = (sprob_recom[ui] / (block.clustsize[ui] as f64))
                    + (sprob_all[i] * (sprob_norecom[ui] / (sprob_first[ui] + 1e-30)));
            }
        }

        var_offset += block.nvar - 1;
    }

    imputed
}

fn main() -> std::io::Result<()> {

    let imputed: Vec<f64> = impute_chunk(0);

    let mut file = File::create(OUTPUT_FILE)?;
    writeln!(file, "{}", imputed.iter().map(|n| n.to_string()).collect::<Vec<String>>().join("\n"))?;

    println!("Imputation result written to {}", OUTPUT_FILE);

    Ok(())
}
