use super::*;
use bitvec::prelude::{BitVec, Lsb0};
use byteorder::{NetworkEndian, ReadBytesExt};
use std::io::{Read, Result};
use std::path::PathBuf;
use std::sync::mpsc::sync_channel;
use std::thread::spawn;

pub struct InputWriter {
    ind_path: PathBuf,
    data_path: PathBuf,
}

impl InputWriter {
    pub fn new(ind_path: &Path, data_path: &Path) -> Self {
        Self {
            ind_path: ind_path.to_owned(),
            data_path: data_path.to_owned(),
        }
    }
}

struct IndexBlockIter<I>(I);

impl<I> Iterator for IndexBlockIter <I>
where I: Iterator<Item = bool> {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> { 
        let mut ind_buffer = BitVec::<Lsb0, u64>::with_capacity(64);
        for _ in 0..64 {
            match self.0.next() {
                Some(new_b) => ind_buffer.push(new_b),
                None => break,
            }
        }
        if ind_buffer.len() > 0 {
            Some(ind_buffer.into_vec()[0])
        } else {
            None
        }
    }
}

impl InputWrite for InputWriter {
    fn write(&mut self, writer: impl Write) -> Result<()> {
        let ind_iter = super::load_ind(&self.ind_path);
        let data_iter = super::load_data(&self.data_path);
        let ind_iter = IndexBlockIter(ind_iter);
        super::write_input(ind_iter, data_iter, writer)
    }
}

pub struct InputReader<R> {
    bound: usize,
    reader: R,
}

impl<R> InputReader<R> {
    pub fn new(bound: usize, reader: R) -> Self {
        assert!(bound >= 64);
        Self { bound, reader }
    }
}

impl<R: Read + Send + 'static> InputRead for InputReader<R> {
    type IndexIterator = impl Iterator<Item = bool>;
    type DataIterator = impl Iterator<Item = Input>;
    fn into_pair_iter(mut self) -> (Self::IndexIterator, Self::DataIterator) {
        let (ind_s, ind_r) = sync_channel(self.bound);
        let (data_s, data_r) = sync_channel(self.bound);
        spawn(move || loop {
            match self.reader.read_u64::<NetworkEndian>() {
                Ok(ind_block) => {
                    let n_ones = ind_block.count_ones() as usize;
                    let n_bytes = (n_ones + 3) / 4;
                    let mut ind_buffer = BitVec::<Lsb0, u64>::from_vec(vec![ind_block]);
                    ind_buffer.resize(64, false);
                    for b in ind_buffer.into_iter() {
                        if ind_s.send(b).is_err() {
                            break;
                        }
                    }
                    let mut symbols: SymbolVec<u8> = BitVec::from_vec(
                        (0..n_bytes)
                            .map(|_| self.reader.read_u8().unwrap())
                            .collect::<Vec<_>>(),
                    )
                    .into();
                    symbols.shrink_to(n_ones);
                    for symbol in symbols.into_iter() {
                        #[cfg(not(feature = "leak-resistant"))]
                        let status = data_s.send(symbol);

                        #[cfg(feature = "leak-resistant")]
                        let status = data_s.send(Input::protect(symbol.into()));

                        if status.is_err() {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        });

        (
            (0..usize::MAX).map(move |_| ind_r.recv().unwrap()),
            (0..usize::MAX).map(move |_| data_r.recv().unwrap()),
        )
    }
}
