pub mod dynamic;
pub mod owned;
pub use dynamic::*;
pub use owned::*;

use crate::symbol::{Symbol, SymbolVec};
use crate::Input;
use bitvec::prelude::{BitVec, Lsb0};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Result, Write};
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

pub fn read_next_input(
    n_ind_left: &mut usize,
    mut reader: impl Read,
) -> Result<(BitVec<Lsb0, u64>, SymbolVec<u8>)> {
    if *n_ind_left == 0 {
        return Err(Error::new(ErrorKind::Other, "No more indices to read"));
    }
    let ind_block = reader.read_u64::<NetworkEndian>()?;
    let n_ones = ind_block.count_ones() as usize;
    let n_bytes = (n_ones + 3) / 4;
    let mut ind_buffer = BitVec::<Lsb0, u64>::from_vec(vec![ind_block]);
    if *n_ind_left >= 64 {
        ind_buffer.resize(64, false);
    } else {
        ind_buffer.resize(*n_ind_left, false);
    }
    let mut symbols: SymbolVec<u8> = BitVec::from_vec(
        (0..n_bytes)
            .map(|_| reader.read_u8().unwrap())
            .collect::<Vec<_>>(),
    )
    .into();
    symbols.shrink_to(n_ones);
    *n_ind_left -= ind_buffer.len();
    Ok((ind_buffer, symbols))
}
