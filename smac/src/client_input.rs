use crate::symbol::Symbol;
use crate::symbol_vec::SymbolVec;
use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use std::fs::{read_dir, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub type Bitmask = BitVec<Lsb0, u64>;

pub fn load_bitmask_iter(bitmask_path: &Path) -> impl Iterator<Item = bool> {
    let f = File::open(bitmask_path).unwrap();
    let f = BufReader::new(f);
    f.lines()
        .map(|line| line.unwrap().parse::<i8>().unwrap() != 0)
}

pub fn load_bitmask(bitmask_path: &Path) -> Bitmask {
    load_bitmask_iter(bitmask_path).collect()
}

pub fn load_symbols_batch(symbols_dir: &Path) -> impl Iterator<Item = (PathBuf, SymbolVec)> {
    read_dir(symbols_dir)
        .unwrap()
        .into_iter()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| !entry.is_dir())
        .map(|entry| {
            let symbols = load_symbols(&entry);
            (entry, symbols)
        })
}

pub fn get_symbols_batch_size(symbols_dir: &Path) -> usize {
    read_dir(symbols_dir)
        .unwrap()
        .into_iter()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| !entry.is_dir())
        .count()
}

pub fn load_symbols_iter(symbols_path: &Path) -> impl Iterator<Item = Symbol> {
    let f = File::open(symbols_path).unwrap();
    let f = BufReader::new(f);
    f.lines()
        .map(|line| line.unwrap().parse::<Symbol>().unwrap())
}

pub fn load_symbols(symbols_path: &Path) -> SymbolVec {
    load_symbols_iter(symbols_path).collect()
}
