use super::*;
use crate::symbol_vec::SymbolVec;
use bitvec::prelude::{BitVec, Lsb0};
use std::io::{Read, Result, Write};
use std::path::Path;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedInput {
    ind: BitVec<Lsb0, u64>,
    data: SymbolVec,
}

impl OwnedInput {
    pub fn from_remote(reader: impl Read) -> Result<Self> {
        bincode::deserialize_from(reader).map_err(|e| Error::new(ErrorKind::Other, e))
    }
    pub fn load(ind_path: &Path, data_path: &Path) -> Self {
        Self {
            ind: Self::load_ind(ind_path),
            data: Self::load_data(data_path),
        }
    }

    fn load_ind(ind_path: &Path) -> BitVec<Lsb0, u64> {
        super::load_ind(ind_path).collect()
    }

    fn load_data(data_path: &Path) -> SymbolVec {
        super::load_data(data_path).collect()
    }
}

impl InputWrite for OwnedInput {
    fn write(&mut self, writer: impl Write) -> Result<()> {
        bincode::serialize_into(writer, self).map_err(|e| Error::new(ErrorKind::Other, e))
    }

    fn stream(&mut self, writer: impl Write) -> Result<()> {
        super::stream_write_input(
            self.ind.len(),
            self.ind.as_slice().iter().cloned(),
            self.data.iter(),
            writer,
        )
    }
}

impl InputRead for OwnedInput {
    type IndexIterator = impl Iterator<Item = bool>;
    type DataIterator = impl Iterator<Item = Input>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator) {
        #[cfg(not(feature = "leak-resistant"))]
        {
            (self.ind.into_iter(), self.data.into_iter())
        }

        #[cfg(feature = "leak-resistant")]
        (
            self.ind.into_iter(),
            // TODO Fix this
            Box::new(self.data.into_iter().map(|v| Input::protect(v as i8))),
        )
    }
}
