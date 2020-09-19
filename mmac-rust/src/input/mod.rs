pub mod dynamic;
pub mod owned;
pub use dynamic::*;
pub use owned::*;

use crate::symbol::{Symbol, SymbolVec};
use crate::Input;
use byteorder::{NetworkEndian, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Result, Write};
use std::path::Path;

pub trait InputWrite {
    fn write(&mut self, writer: impl Write) -> Result<()>;
}

pub trait InputRead {
    type IndexIterator: Iterator<Item = bool>;
    type DataIterator: Iterator<Item = Input>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator);
}

pub fn load_ind(ind_path: &Path) -> impl Iterator<Item = bool> {
    let f = File::open(ind_path).unwrap();
    let f = BufReader::new(f);
    f.lines()
        .map(|line| line.unwrap().parse::<i8>().unwrap() != 0)
}

pub fn load_data(data_path: &Path) -> impl Iterator<Item = Symbol> {
    let f = File::open(data_path).unwrap();
    let f = BufReader::new(f);
    f.lines()
        .map(|line| line.unwrap().parse::<Symbol>().unwrap())
}

pub fn write_input(
    n_ind: usize,
    ind_block_iter: impl Iterator<Item = u64>,
    mut data_iter: impl Iterator<Item = Symbol>,
    mut writer: impl Write,
) -> Result<()> {
    writer.write_u32::<NetworkEndian>(n_ind as u32)?;
    for v in ind_block_iter {
        let n_ones = v.count_ones();
        let mut data_buffer = SymbolVec::<u8>::with_capacity(n_ones as usize);
        for _ in 0..n_ones {
            data_buffer.push(data_iter.next().unwrap());
        }
        writer.write_u64::<NetworkEndian>(v)?;
        debug_assert_eq!(data_buffer.as_slice().len(), (n_ones as usize + 3) / 4);
        for &v in data_buffer.as_slice() {
            writer.write_u8(v)?;
        }
    }
    Ok(())
}
