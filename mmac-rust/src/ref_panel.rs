use crate::Real;
use bitvec::prelude::{bitvec, BitVec, Lsb0};
use ndarray::Array1;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufRead, BufReader, Result};
use std::path::Path;

pub struct Block {
    pub indmap: Array1<usize>,
    pub nvar: usize,
    pub nuniq: usize,
    pub clustsize: Array1<Real>,
    pub rhap: Vec<BitVec>,
    //pub eprob: Array1<f64>,
    pub rprob: Array1<f64>,
    pub afreq: Array1<f64>,
}

impl Block {
    pub fn read(m: usize, mut lines_iter: impl Iterator<Item = Result<String>>) -> Self {
        // read block metadata
        let line = lines_iter.next().unwrap().unwrap();
        let mut iter = line.split_ascii_whitespace();
        let tok = iter.nth(7).unwrap(); // info field
        let tok = tok.split(";").collect::<Vec<_>>();

        let mut nvar = None;
        let mut nuniq = None;

        for t in tok {
            let t = t.split("=").collect::<Vec<_>>();
            match t[0] {
                "VARIANTS" => nvar = Some(t[1].parse::<usize>().unwrap()),
                "REPS" => nuniq = Some(t[1].parse::<usize>().unwrap()),
                _ => continue,
            }
        }

        let nvar = nvar.unwrap();
        let nuniq = nuniq.unwrap();

        iter.next().unwrap(); // skip one column
        let indmap = iter
            .map(|s| s.parse::<usize>().unwrap())
            .collect::<Vec<_>>();

        let mut clustsize = Array1::<usize>::zeros(nuniq);
        indmap.iter().for_each(|&v| clustsize[v] += 1);

        //let mut eprob = Vec::<f64>::with_capacity(nvar);
        let mut rprob = Vec::<f64>::with_capacity(nvar);
        let mut rhap: Vec<BitVec> = Vec::new();
        let mut afreq = Vec::<f64>::with_capacity(nvar);

        // read block data
        for _ in 0..nvar {
            let line = lines_iter.next().unwrap().unwrap();
            let mut iter = line.split_ascii_whitespace();
            let tok = iter.nth(7).unwrap(); // info field
            let tok = tok.split(";").collect::<Vec<_>>();

            //let mut new_eprob = None;
            let mut new_rprob = None;

            for t in tok {
                let t = t.split("=").collect::<Vec<_>>();
                match t[0] {
                    //"Err" => new_eprob = Some(t[1].parse::<f64>().unwrap()),
                    "Recom" => new_rprob = Some(t[1].parse::<f64>().unwrap()),
                    _ => continue,
                }
            }

            //eprob.push(new_eprob.unwrap());
            rprob.push(new_rprob.unwrap());

            let data = iter.next().unwrap(); // data for one variant
            let mut alt_count = 0;

            let mut new_rhap_row = bitvec![Lsb0, usize; 0; nuniq];
            data.chars()
                .zip(new_rhap_row.as_mut_bitslice())
                .enumerate()
                .for_each(|(ind, (b, mut r))| {
                    let geno = match b {
                        '0' => 0,
                        '1' => 1,
                        _ => panic!("Invalid file format"),
                    };
                    //rhap[[cur_var, ind]] = geno;
                    *r = geno == 1;
                    if geno == 1 {
                        alt_count += clustsize[ind];
                    }
                });
            rhap.push(new_rhap_row);
            afreq.push(
                f64::from(u32::try_from(alt_count).unwrap()) / f64::from(u32::try_from(m).unwrap()),
            );
        }

        Self {
            indmap: Array1::from(indmap),
            nvar,
            nuniq,
            clustsize: Array1::from(
                clustsize
                    .into_iter()
                    .map(|&v| (v as u32).into())
                    .collect::<Vec<_>>(),
            ),
            rhap,
            //eprob: Array1::from(eprob),
            rprob: Array1::from(rprob),
            afreq: Array1::from(afreq),
        }
    }
}

pub struct RefPanel {
    pub n_haps: usize,
    pub n_markers: usize,
    pub blocks: Vec<Block>,
}

impl RefPanel {
    /// integers to large genomic windows which are
    /// imputed independently
    /// TODO: chunk_id is currently ignored
    /// and the entire toy data is loaded
    pub fn load(_chunk_id: usize, ref_panel_path: &Path) -> Self {
        let f = File::open(ref_panel_path).expect("Unable to open reference file");
        let f = BufReader::new(f);

        let mut lines_iter = f.lines();

        let (n_blocks, n_haps, n_markers) = Self::read_metadata(&mut lines_iter);

        let mut blocks = Vec::with_capacity(n_blocks);

        // read all blocks
        for _ in 0..n_blocks {
            blocks.push(Block::read(n_haps, &mut lines_iter));
        }
        Self {
            n_haps,
            n_markers,
            blocks,
        }
    }

    /// read metadata from file header
    fn read_metadata(
        mut lines_iter: impl Iterator<Item = Result<String>>,
    ) -> (usize, usize, usize) {
        let mut n_blocks: Option<usize> = None;
        let mut n_haps: Option<usize> = None;
        let mut n_markers: Option<usize> = None;

        loop {
            let line = lines_iter.next().unwrap().unwrap();
            if &line[..2] == "##" {
                let tok = line[2..].split("=").collect::<Vec<_>>();
                match tok[0] {
                    "n_blocks" => n_blocks = Some(tok[1].parse::<usize>().unwrap()),
                    "n_haps" => n_haps = Some(tok[1].parse::<usize>().unwrap()),
                    "n_markers" => n_markers = Some(tok[1].parse::<usize>().unwrap()),
                    _ => continue,
                }
            } else if &line[..1] == "#" {
                // data header
                break;
            }
        }

        let n_blocks = n_blocks.unwrap();
        let n_haps = n_haps.unwrap();
        let n_markers = n_markers.unwrap();
        (n_blocks, n_haps, n_markers)
    }
}
