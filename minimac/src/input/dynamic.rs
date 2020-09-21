use super::*;
use bitvec::prelude::{BitVec, Lsb0};
use byteorder::{NetworkEndian, ReadBytesExt};
use crossbeam::{bounded, Receiver, Sender};
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
        let mut owned = OwnedInput::load(&self.ind_path, &self.data_path);
        owned.write(writer)
    }

    fn stream(&mut self, writer: impl Write) -> Result<()> {
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
        super::stream_write_input(self.n_ind, ind_iter, data_iter, writer)
    }
}

pub struct InputReader<R> {
    n_ind: usize,
    capacity: usize,
    reader: Arc<Mutex<R>>,
}

impl<R: Read + Send + 'static> InputReader<R> {
    pub fn new(capacity: usize, reader: Arc<Mutex<R>>) -> Self {
        let n_ind = reader.lock().unwrap().read_u32::<NetworkEndian>().unwrap() as usize;
        Self { n_ind, capacity, reader }
    }
}

impl<R: Read + Send + 'static> InputRead for InputReader<R> {
    type IndexIterator = impl Iterator<Item = bool>;
    type DataIterator = impl Iterator<Item = Input>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator) {
        let n_ind_left = Arc::new(Mutex::new(self.n_ind));
        let (send_ind, recv_ind) = bounded(self.capacity);
        let (send_data, recv_data) = bounded(self.capacity);
        IndexIter::fill_buffer(
            send_ind.clone(),
            send_data.clone(),
            n_ind_left.clone(),
            self.reader.clone(),
        );
        (
            IndexIter {
                buffer: None, 
                reader: self.reader,
                n_ind_left,
                send_ind,
                send_data,
                recv_ind,
            },
            DataIter {
                buffer: None, 
                recv_data,
            },
        )
    }
}

pub struct IndexIter<R> {
    buffer: Option<bitvec::vec::IntoIter<Lsb0, u64>>,
    reader: Arc<Mutex<R>>,
    n_ind_left: Arc<Mutex<usize>>,
    send_ind: Sender<Option<BitVec<Lsb0, u64>>>,
    send_data: Sender<Option<SymbolVec>>,
    recv_ind: Receiver<Option<BitVec<Lsb0, u64>>>,
}

impl<R: Read + Send + 'static> IndexIter<R> {
    fn fill_buffer(
        send_ind: Sender<Option<BitVec<Lsb0, u64>>>,
        send_data: Sender<Option<SymbolVec>>,
        n_ind_left: Arc<Mutex<usize>>,
        reader: Arc<Mutex<R>>,
    ) {
        if send_ind.is_full() || send_data.is_full() {
            return;
        }
        rayon::spawn(move || {
            if let Ok(mut reader) = reader.lock() {
                let mut n_ind_left = n_ind_left.lock().unwrap();
                for _ in 0..5 {
                    if send_ind.is_full() || send_data.is_full() {
                        return;
                    }
                    if let Ok((ind, data)) = stream_read_next_input(&mut *n_ind_left, &mut *reader)
                    {
                        if send_ind.send(Some(ind)).is_err() {
                            break;
                        }
                        if send_data.send(Some(data)).is_err() {
                            break;
                        }
                    } else {
                        let _ = send_ind.send(None);
                        let _ = send_data.send(None);
                        break;
                    }
                }
            }
        });
    }
}

impl<R: Read + Send + 'static> Iterator for IndexIter<R> {
    type Item = bool;
    fn next(&mut self) -> Option<bool> {
        if self.buffer.is_none() {
            self.buffer = Some(self.recv_ind.recv().unwrap()?.into_iter());
        }
        match self.buffer.as_mut().unwrap().next() {
            Some(b) => Some(b),
            None => {
                Self::fill_buffer(
                    self.send_ind.clone(),
                    self.send_data.clone(),
                    self.n_ind_left.clone(),
                    self.reader.clone(),
                );
                self.buffer = Some(self.recv_ind.recv().unwrap()?.into_iter());
                self.next()
            }
        }
    }
}

pub struct DataIter {
    buffer: Option<crate::symbol_vec::IntoIter>,
    recv_data: Receiver<Option<SymbolVec>>,
}

impl Iterator for DataIter {
    type Item = Input;
    fn next(&mut self) -> Option<Input> {
        if self.buffer.is_none() {
            self.buffer = Some(self.recv_data.recv().unwrap()?.into_iter());
        }
        match self.buffer.as_mut().unwrap().next() {
            Some(symbol) => {
                #[cfg(not(feature = "leak-resistant"))]
                {
                    Some(symbol)
                }

                #[cfg(feature = "leak-resistant")]
                Some(Input::protect(symbol.into()))
            }
            None => {
                self.buffer = Some(self.recv_data.recv().unwrap()?.into_iter());
                self.next()
            }
        }
    }
}
