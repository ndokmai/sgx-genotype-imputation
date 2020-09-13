use crate::Real;
use bitvec::prelude::{bitvec, BitVec, Lsb0};
use ndarray::Array1;
use std::convert::TryFrom;
use std::io::Result;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub indmap: Array1<u16>,
    pub nvar: usize,
    pub nuniq: usize,
    pub clustsize: Array1<Real>,
    pub rhap: Vec<BitVec>,
    //pub eprob: Array1<f64>,
    pub rprob: Array1<f32>,
    pub afreq: Array1<f32>,
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
        let indmap = iter.map(|s| s.parse::<u16>().unwrap()).collect::<Vec<_>>();

        let mut clustsize = Array1::<u16>::zeros(nuniq);
        indmap.iter().for_each(|&v| clustsize[v as usize] += 1);

        //let mut eprob = Vec::<f64>::with_capacity(nvar);
        let mut rprob = Vec::<f32>::with_capacity(nvar);
        let mut rhap: Vec<BitVec> = Vec::new();
        let mut afreq = Vec::<f32>::with_capacity(nvar);

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
                    "Recom" => new_rprob = Some(t[1].parse::<f32>().unwrap()),
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
                    *r = geno == 1;
                    if geno == 1 {
                        alt_count += clustsize[ind];
                    }
                });
            rhap.push(new_rhap_row);
            afreq.push(
                f32::from(u16::try_from(alt_count).unwrap()) / f32::from(u16::try_from(m).unwrap()),
            );
        }

        Self {
            indmap: Array1::from(indmap),
            nvar,
            nuniq,
            clustsize: Array1::from(clustsize.into_iter().map(|&v| v.into()).collect::<Vec<_>>()),
            rhap,
            //eprob: Array1::from(eprob),
            rprob: Array1::from(rprob),
            afreq: Array1::from(afreq),
        }
    }
}
