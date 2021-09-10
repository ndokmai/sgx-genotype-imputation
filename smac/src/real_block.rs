use crate::Real;
use bitvec::prelude::BitVec;
use m3vcf::Block;
use ndarray::Array1;

#[derive(Clone)]
pub struct RealBlock {
    pub indmap: Array1<u16>,
    pub nvar: usize,
    pub nuniq: usize,
    pub clustsize: Array1<Real>,
    pub rhap: Vec<BitVec>,
    pub rprob: Array1<f32>,
    pub afreq: Array1<f32>,
}

impl From<Block> for RealBlock {
    fn from(block: Block) -> Self {
        let clustsize =
            Array1::<Real>::from_shape_fn(block.clustsize.dim(), |i| block.clustsize[i].into());
        Self {
            indmap: block.indmap,
            nvar: block.nvar,
            nuniq: block.nuniq,
            clustsize,
            rhap: block.rhap,
            rprob: block.rprob,
            afreq: block.afreq,
        }
    }
}
