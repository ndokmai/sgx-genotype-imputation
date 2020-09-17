use super::*;
use crate::symbol::{Symbol, SymbolVec};
use bitvec::prelude::{BitVec, Lsb0};
use byteorder::{NetworkEndian, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Result, Write};
use std::path::Path;

#[derive(Clone)]
pub struct OwnedInput {
    ind: BitVec<Lsb0, u64>,
    data: SymbolVec<u64>,
}

impl OwnedInput {
    pub fn load(ind_path: &Path, data_path: &Path) -> Self {
        Self {
            ind: Self::load_ind(ind_path),
            data: Self::load_data(data_path),
        }
    }

    fn load_ind(ind_path: &Path) -> BitVec<Lsb0, u64> {
        let f = File::open(ind_path).unwrap();
        let f = BufReader::new(f);
        f.lines()
            .map(|line| line.unwrap().parse::<i8>().unwrap() != 0)
            .collect()
    }

    fn load_data(data_path: &Path) -> SymbolVec<u64> {
        let f = File::open(data_path).unwrap();
        let f = BufReader::new(f);
        f.lines()
            .map(|line| line.unwrap().parse::<Symbol>().unwrap())
            .collect()
    }
}

impl InputWriter for OwnedInput {
    fn write(&mut self, mut writer: impl Write) -> Result<()> {
        let mut data_iter = self.data.iter();
        for &v in self.ind.as_slice() {
            let n_ones = v.count_ones();
            let mut data_buffer = SymbolVec::<u8>::new();
            for _ in 0..n_ones {
                data_buffer.push(data_iter.next().unwrap());
            }
            data_buffer.shrink_to_fit();
            writer.write_u64::<NetworkEndian>(v)?;
            for &v in data_buffer.as_slice() {
                writer.write_u8(v)?;
            }
        }
        Ok(())
    }
}

#[cfg(not(feature = "leak-resistant"))]
impl InputReader for OwnedInput {
    type IndexIterator = bitvec::vec::IntoIter<Lsb0, u64>;
    type DataIterator = crate::symbol::IntoIter<u64>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator) {
        (self.ind.into_iter(), self.data.into_iter())
    }
}

#[cfg(feature = "leak-resistant")]
impl InputReader for OwnedInput {
    type IndexIterator = bitvec::vec::IntoIter<Lsb0, u64>;
    type DataIterator = Box<dyn Iterator<Item = Input>>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator) {
        (
            self.ind.into_iter(),
            // TODO Fix this
            Box::new(self.data.into_iter().map(|v| Input::protect(v as i8))),
        )
    }
}
