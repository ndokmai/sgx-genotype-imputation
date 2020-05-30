use crate::symbol::Symbol;
use ndarray::{Array1, Array2, Array3};

pub struct Params {
    pub init: Array1<f64>,
    pub emit: Array3<f64>,
    pub tran: Array2<f64>,
    pub num_refs: usize
}

impl Params {
    pub fn init(refs: &[Vec<Symbol>], mg_len: usize) -> Self {
        let num_refs = refs.len();
        let mut init = Array1::<f64>::zeros(num_refs);
        let mut emit = Array3::<f64>::zeros((5, num_refs, mg_len));
        let mut tran = Array2::<f64>::zeros((num_refs, num_refs));

        eprintln!("Setting initial probabilities ...");
        init.fill(1. / (num_refs as f64));

        eprintln!("Setting transition probabilities ...");
        tran.fill((1. - 0.6) / (num_refs as f64));
        tran.diag_mut().fill(0.6);

        eprintln!("Setting emission probabilities ...");
        for i in 0..5 {
            for j in 0..num_refs {
                for k in 0..mg_len {
                    if i == 4 {
                        emit[[i, j, k]] = 1.0;
                    } else if refs[j][k].pos() == i {
                        emit[[i, j, k]] = 0.60;
                    } else {
                        emit[[i, j, k]] = 0.40;
                    }
                }
            }
        }
        Self { init, emit, tran, num_refs }
    }
}
