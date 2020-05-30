use crate::symbol::Symbol;
use ndarray::Array2;
use std::io::{BufRead, BufReader, Read, Result};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};

/// Asynchronous input reader
pub struct InputFeed<R: Read + Send + 'static> {
    stream: Arc<Mutex<BufReader<R>>>,
}

impl<R: Read + Send + 'static> InputFeed<R> {
    pub fn new(stream: BufReader<R>) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
        }
    }

    /// Asynchronous input reading.
    /// Need Array2 for contiguous memory.
    pub fn take(&self, n: usize) -> JoinHandle<Result<Array2<Symbol>>> {
        assert!(n >= 1);
        let stream = self.stream.clone();
        spawn(move || {
            let mut buffer = Vec::new();
            let mut stream = stream.lock().unwrap();
            let mut nrows = 0;
            for _ in 0..n {
                let mut line = String::new();
                match stream.read_line(&mut line) {
                    Ok(0) => {
                        break;
                    }
                    Ok(_) => {
                        let mut parsed = line
                            .trim_end()
                            .chars()
                            .map(|c| Symbol::parse(&c).unwrap())
                            .collect::<Vec<_>>();
                        buffer.append(&mut parsed);
                        nrows += 1;
                    }
                    Err(e) => return Err(e),
                }
            }
            let ncols = buffer.len() / nrows;
            Ok(Array2::from_shape_vec((nrows, ncols), buffer).unwrap())
        })
    }
}
