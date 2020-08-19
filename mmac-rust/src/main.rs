mod input;
mod ref_panel;
use crate::input::load_chunk_from_input;
use crate::ref_panel::RefPanel;
use ndarray::Array2;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::time::Instant;

static REF_FILE: &'static str = "largeref.m3vcf";
static INPUT_FILE: &'static str = "input.txt";
static OUTPUT_FILE: &'static str = "output.txt";

fn impute_chunk(_chunk_id: usize, thap: &[i8], ref_panel: &RefPanel) -> Vec<f64> {
    let blocks = &ref_panel.blocks;
    let m = ref_panel.n_haps;

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
            let emi = if tsym == block.rhap[[0, block.indmap[i]]] {
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
            fwdcache_all[[b, i]] = sprob_all[i]; // save cache
        }

        let sprob_first = sprob.to_vec();
        let mut sprob_norecom = sprob.to_vec();

        // Walk
        for j in 1..block.nvar {
            let rec = block.rprob[j - 1];
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
                sprob[i] = complement * sprob[i] + (block.clustsize[i] as f64) * sprob_tot;
            }

            // Emission
            if tsym != -1 {
                for i in 0..block.nuniq {
                    let emi = if tsym == block.rhap[[j, i]] {
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
                fwdprob[[j, i]] = sprob[i];
                fwdprob_norecom[[j, i]] = sprob_norecom[i];
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
        if b < blocks.len() - 1 {
            // Skip last block
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
            jprob[block.indmap[i]] += fwdcache_all[[b, i]] * sprob_all[i];
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
            let rec = block.rprob[j - 1];
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
                let combined = jprob[i]
                    * (fwdprob_norecom[[j, i]] * sprob_norecom[i]
                        / (fwdprob_first[i] * sprob_first[i] + 1e-30))
                    + (fwdprob[[j, i]] * sprob[i] - fwdprob_norecom[[j, i]] * sprob_norecom[i])
                        / (block.clustsize[i] as f64);

                let rsym = block.rhap[[j, i]];
                if rsym == 1 {
                    p1 += combined;
                } else if rsym == 0 {
                    p0 += combined;
                }
            }
            imputed[varind] = p1 / (p1 + p0);

            // Emission
            if tsym > -1 {
                // not missing
                for i in 0..block.nuniq {
                    let emi = if tsym == block.rhap[[j, i]] {
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
                sprob[i] = complement * sprob[i] + (block.clustsize[i] as f64) * sprob_tot;
            }

            // Impute very first position (edge case)
            // TODO: refactor with the same code block above
            if b == 0 && j == 1 {
                let mut p1 = 0.0;
                let mut p0 = 0.0;
                for i in 0..block.nuniq {
                    let combined = jprob[i]
                        * (fwdprob_norecom[[0, i]] * sprob_norecom[i]
                            / (fwdprob_first[i] * sprob_first[i] + 1e-30))
                        + (fwdprob[[0, i]] * sprob[i] - fwdprob_norecom[[0, i]] * sprob_norecom[i])
                            / (block.clustsize[i] as f64);

                    let rsym = block.rhap[[0, i]];
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

fn main() {
    let chunk_id = 0;
    let ref_panel_path = Path::new(REF_FILE);
    let input_path = Path::new(INPUT_FILE);

    eprintln!(
        "Loading chunk {} from reference panel ({})",
        chunk_id, REF_FILE
    );

    let now = std::time::Instant::now();
    let ref_panel = RefPanel::load(chunk_id, &ref_panel_path);
    eprintln!(
        "Reference panel load time: {} ms",
        (Instant::now() - now).as_millis()
    );

    eprintln!("n_blocks = {}", ref_panel.blocks.len());
    eprintln!("n_haps = {}", ref_panel.n_haps);
    eprintln!("n_markers = {}", ref_panel.n_markers);

    eprintln!("Loading chunk {} from input ({})", chunk_id, INPUT_FILE);
    let now = std::time::Instant::now();
    let thap = load_chunk_from_input(chunk_id, &input_path);
    eprintln!("Input load time: {} ms", (Instant::now() - now).as_millis());

    let now = std::time::Instant::now();
    let imputed: Vec<f64> = impute_chunk(chunk_id, thap.as_slice(), &ref_panel);
    eprintln!("Imputation time: {} ms", (Instant::now() - now).as_millis());

    let mut file = File::create(OUTPUT_FILE).unwrap();
    writeln!(
        file,
        "{}",
        imputed
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    )
    .unwrap();

    eprintln!("Imputation result written to {}", OUTPUT_FILE);
}
