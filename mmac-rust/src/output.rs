use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use std::iter::Rev;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

pub trait OutputWrite<T>
where
    T: serde::Serialize,
{
    fn push(&mut self, v: T);
}

impl<T, O> OutputWrite<T> for &mut O
where
    O: OutputWrite<T>,
    T: serde::Serialize,
{
    fn push(&mut self, v: T) {
        (*self).push(v);
    }
}

pub trait OutputRead<T>: Iterator<Item = T>
where
    T: for<'de> serde::Deserialize<'de>,
{
}

pub struct OwnedOutputWriter<T>(Vec<T>);

impl<T> OwnedOutputWriter<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn into_reader(self) -> OwnedOutputReader<T> {
        self.0.into_iter().rev()
    }
}

impl<T> OutputWrite<T> for OwnedOutputWriter<T>
where
    T: serde::Serialize,
{
    fn push(&mut self, v: T) {
        self.0.push(v);
    }
}

pub type OwnedOutputReader<T> = Rev<std::vec::IntoIter<T>>;

impl<T> OutputRead<T> for OwnedOutputReader<T> where T: for<'de> serde::Deserialize<'de> {}

pub struct StreamOutputWriter<W>(W);

impl<W: Write> StreamOutputWriter<W> {
    pub fn new(n_outputs: usize, mut writer: W) -> Self {
        writer.write_u32::<NetworkEndian>(n_outputs as u32).unwrap();
        Self(writer)
    }
}

impl<W, T> OutputWrite<T> for StreamOutputWriter<W>
where
    W: Write,
    T: serde::Serialize,
{
    fn push(&mut self, v: T) {
        bincode::serialize_into(&mut self.0, &v).unwrap();
    }
}

pub struct StreamOutputReader<R, T> {
    buffer: Vec<T>,
    _phantom: PhantomData<R>,
}

impl<R, T> StreamOutputReader<R, T>
where
    R: Read,
    T: for<'de> serde::Deserialize<'de>,
{
    pub fn read(mut reader: R) -> Self {
        let n_outputs = reader.read_u32::<NetworkEndian>().unwrap() as usize;
        let mut buffer = Vec::new();
        for _ in 0..n_outputs {
            let v = bincode::deserialize_from(&mut reader).unwrap();
            buffer.push(v);
        }
        Self {
            buffer,
            _phantom: PhantomData,
        }
    }
}

impl<R, T> Iterator for StreamOutputReader<R, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.buffer.pop()
    }
}

impl<T, R> OutputRead<T> for StreamOutputReader<R, T> where T: for<'de> serde::Deserialize<'de> {}

pub struct MutexStreamOutputWriter<W> {
    n_outputs: Option<usize>,
    writer: Arc<Mutex<W>>,
}

impl<W: Write> MutexStreamOutputWriter<W> {
    pub fn new(n_outputs: usize, writer: Arc<Mutex<W>>) -> Self {
        Self {
            n_outputs: Some(n_outputs),
            writer,
        }
    }
}

impl<W, T> OutputWrite<T> for MutexStreamOutputWriter<W>
where
    W: Write,
    T: serde::Serialize,
{
    fn push(&mut self, v: T) {
        //println!("count = {}", Arc::strong_count(&self.0));
        //println!("lockable = {}", self.0.try_lock().is_ok());
        let mut stream = self.writer.lock().unwrap();
        if self.n_outputs.is_some() {
            stream
                .write_u32::<NetworkEndian>(self.n_outputs.take().unwrap() as u32)
                .unwrap();
        }
        bincode::serialize_into(&mut *stream, &v).unwrap();
    }
}
