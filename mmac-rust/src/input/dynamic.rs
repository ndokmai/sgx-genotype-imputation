use super::*;
use bitvec::prelude::{BitVec, Lsb0};
use byteorder::{NetworkEndian, ReadBytesExt};
use crossbeam::{bounded, Sender};
use std::io::{Read, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
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
    bound: usize,
    reader: Arc<Mutex<R>>,
}

impl<R: Read + Send + 'static> InputReader<R> {
    pub fn new(bound: usize, reader: Arc<Mutex<R>>) -> Self {
        let n_ind = reader.lock().unwrap().read_u32::<NetworkEndian>().unwrap() as usize;
        Self {
            n_ind,
            bound,
            reader,
        }
    }

    pub fn fill_buffer(
        n_ind_left: Arc<AtomicUsize>,
        send_ind: Sender<bool>,
        send_data: Sender<Input>,
        reader: Arc<Mutex<R>>,
    ) {
        if n_ind_left.load(Ordering::Relaxed) == 0 {
            return;
        }
        if send_ind.is_full() || send_data.is_full() {
            return;
        }
        if reader.try_lock().is_err() {
            return;
        }
        rayon::spawn(move || {
            if let Ok(mut reader) = reader.try_lock() {
                if let Ok(ind_block) = reader.read_u64::<NetworkEndian>() {
                    let n_ones = ind_block.count_ones() as usize;
                    let n_bytes = (n_ones + 3) / 4;
                    let mut ind_buffer = BitVec::<Lsb0, u64>::from_vec(vec![ind_block]);
                    ind_buffer.resize(64, false);
                    let mut symbols: SymbolVec<u8> = BitVec::from_vec(
                        (0..n_bytes)
                            .map(|_| reader.read_u8().unwrap())
                            .collect::<Vec<_>>(),
                    )
                    .into();
                    symbols.shrink_to(n_ones);
                    for symbol in symbols.into_iter() {
                        #[cfg(not(feature = "leak-resistant"))]
                        let status = send_data.send(symbol);

                        #[cfg(feature = "leak-resistant")]
                        let status = send_data.send(Input::protect(symbol.into()));

                        if status.is_err() {
                            break;
                        }
                    }
                    let mut n_ind_left_dec = n_ind_left.load(Ordering::Relaxed);
                    for b in ind_buffer.into_iter() {
                        if send_ind.send(b).is_err() {
                            break;
                        }
                        n_ind_left_dec -= 1;
                        if n_ind_left_dec == 0 {
                            break;
                        }
                    }
                    n_ind_left.store(n_ind_left_dec, Ordering::Relaxed);
                }
            }
        });
    }
}

impl<R: Read + Send + 'static> InputRead for InputReader<R> {
    type IndexIterator = impl Iterator<Item = bool>;
    type DataIterator = impl Iterator<Item = Input>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator) {
        let (send_ind, recv_ind) = bounded(self.bound);
        let (send_data, recv_data) = bounded(self.bound);

        let n_ind_left = Arc::new(AtomicUsize::new(self.n_ind));

        Self::fill_buffer(
            n_ind_left.clone(),
            send_ind.clone(),
            send_data.clone(),
            self.reader.clone(),
        );

        let ind_iter = (0..)
            .map(move |_| {
                Self::fill_buffer(
                    n_ind_left.clone(),
                    send_ind.clone(),
                    send_data.clone(),
                    self.reader.clone(),
                );
                recv_ind.recv()
            })
            .take_while(|v| v.is_ok())
            .map(|v| v.unwrap());

        (
            ind_iter,
            (0..)
                .map(move |_| recv_data.recv())
                .take_while(|v| v.is_ok())
                .map(|v| v.unwrap()),
        )
    }
}
