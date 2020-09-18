use std::io::{Read, Write};
use std::iter::Rev;
use std::marker::PhantomData;

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
        let mut buffer = Vec::new();
        loop {
            if let Ok(v) = bincode::deserialize_from(&mut reader) {
                buffer.push(v);
            } else {
                break;
            }
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
