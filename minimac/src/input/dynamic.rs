use super::*;
use bitvec::prelude::{BitVec, Lsb0};
use byteorder::{NetworkEndian, ReadBytesExt};
use crossbeam::{unbounded, Receiver, Sender};
use std::io::{Read, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct InputWriter {
    n_ind: usize,
    ind_path: PathBuf,
    data_path: PathBuf,
}

impl InputWriter {
    pub fn new(n_ind: usize, ind_path: &Path, data_path: &Path) -> Self {
        Self {
            n_ind,
            ind_path: ind_path.to_owned(),
            data_path: data_path.to_owned(),
        }
    }
}

impl InputWrite for InputWriter {
    fn write(&mut self, writer: impl Write) -> Result<()> {
        let mut ind_iter = super::load_ind(&self.ind_path);
        let data_iter = super::load_data(&self.data_path);
        let ind_iter = (0..)
            .map(move |_| {
                let mut ind_buffer = BitVec::<Lsb0, u64>::with_capacity(64);
                for _ in 0..64 {
                    match ind_iter.next() {
                        Some(new_b) => ind_buffer.push(new_b),
                        None => break,
                    }
                }
                if ind_buffer.len() > 0 {
                    Some(ind_buffer.into_vec()[0])
                } else {
                    None
                }
            })
            .take_while(|v| v.is_some())
            .map(|v| v.unwrap());
        super::write_input(self.n_ind, ind_iter, data_iter, writer)
    }
}

pub struct InputReader<R> {
    n_ind: usize,
    reader: Arc<Mutex<R>>,
}

impl<R: Read + Send + 'static> InputReader<R> {
    pub fn new(reader: Arc<Mutex<R>>) -> Self {
        let n_ind = reader.lock().unwrap().read_u32::<NetworkEndian>().unwrap() as usize;
        Self { n_ind, reader }
    }
}

impl<R: Read + Send + 'static> InputRead for InputReader<R> {
    type IndexIterator = impl Iterator<Item = bool>;
    type DataIterator = impl Iterator<Item = Input>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator) {
        let (sender, receiver) = unbounded();
        let mut n_ind_left = self.n_ind;
        let (inds, symbols) =
            super::read_next_input(&mut n_ind_left, &mut *self.reader.lock().unwrap()).unwrap();
        (
            IndexIter {
                buffer: inds.into_iter(),
                reader: self.reader,
                n_ind_left,
                sender,
            },
            DataIter {
                buffer: symbols.into_iter(),
                receiver,
            },
        )
    }
}

pub struct IndexIter<R> {
    buffer: bitvec::vec::IntoIter<Lsb0, u64>,
    reader: Arc<Mutex<R>>,
    n_ind_left: usize,
    sender: Sender<SymbolVec>,
}

impl<R: Read> Iterator for IndexIter<R> {
    type Item = bool;
    fn next(&mut self) -> Option<bool> {
        match self.buffer.next() {
            Some(b) => Some(b),
            None => {
                let (inds, symbols) =
                    super::read_next_input(&mut self.n_ind_left, &mut *self.reader.lock().unwrap())
                        .ok()?;
                self.buffer = inds.into_iter();
                self.sender.send(symbols).unwrap();
                self.next()
            }
        }
    }
}

pub struct DataIter {
    buffer: crate::symbol_vec::IntoIter,
    receiver: Receiver<SymbolVec>,
}

impl Iterator for DataIter {
    type Item = Input;
    fn next(&mut self) -> Option<Input> {
        match self.buffer.next() {
            Some(symbol) => {
                #[cfg(not(feature = "leak-resistant"))]
                {
                    Some(symbol)
                }

                #[cfg(feature = "leak-resistant")]
                Some(Input::protect(symbol.into()))
            }
            None => {
                self.buffer = self.receiver.recv().ok()?.into_iter();
                self.next()
            }
        }
    }
}
