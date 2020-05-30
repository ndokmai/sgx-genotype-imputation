use crate::symbol::Symbol;
use ndarray::{Array1, Array2, Array3};
use ndarray::{ArrayView1, ArrayView2, ArrayView3};

pub struct Params {
    pub init: Array1<f64>,
    pub emit: Array3<f64>,
    pub tran: Array2<f64>,
    pub nrefs: usize,
}

impl Params {
    pub fn init_test_params(refs: &[Vec<Symbol>], input_len: usize) -> Self {
        let nrefs = refs.len();
        let mut init = unsafe { Array1::<f64>::uninitialized(nrefs) };
        let mut emit = unsafe { Array3::<f64>::uninitialized((5, nrefs, input_len)) };
        let mut tran = unsafe { Array2::<f64>::uninitialized((nrefs, nrefs)) };

        init.fill(1. / (nrefs as f64));

        tran.fill((1. - 0.6) / (nrefs as f64));
        tran.diag_mut().fill(0.6);

        for i in 0..5 {
            for j in 0..nrefs {
                for k in 0..input_len {
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
        init = init.map(|x| x.ln());
        emit = emit.map(|x| x.ln());
        tran = tran.map(|x| x.ln());
        Self {
            init,
            emit,
            tran,
            nrefs,
        }
    }

    pub fn get_views(&self) -> (ArrayView1<f64>, ArrayView3<f64>, ArrayView2<f64>) {
        (self.init.view(), self.emit.view(), self.tran.view())
    }
}
