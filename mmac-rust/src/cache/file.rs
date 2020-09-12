use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{remove_file, File};
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::{spawn, JoinHandle};

static FILE_ROOT: &str = "/tmp";
const WRITER_BUF_SIZE: usize = 1 << 24;

// TODO Delete files upon error
fn offload_proc<T>(r: Receiver<T>, s: SyncSender<T>)
where
    T: Send + 'static + Serialize + for<'de> Deserialize<'de>,
{
    let cache_filename = format!("{}.cache", rand::random::<u64>());
    let cache_path = Path::new(FILE_ROOT).join(cache_filename);
    let mut bytes_positions = Vec::new();
    {
        let mut cache_file =
            BufWriter::with_capacity(WRITER_BUF_SIZE, File::create(&cache_path).unwrap());
        loop {
            match r.recv() {
                Ok(v) => {
                    bytes_positions.push(cache_file.stream_position().unwrap());
                    bincode::serialize_into(&mut cache_file, &v).unwrap();
                }
                Err(_) => break,
            }
        }
        cache_file.flush().unwrap();
    }

    let mut cache_file = BufReader::new(File::open(&cache_path).unwrap());
    for pos in bytes_positions.into_iter().rev() {
        cache_file.seek(SeekFrom::Start(pos)).unwrap();
        let v: T = bincode::deserialize_from(&mut cache_file).unwrap();
        s.send(v).unwrap();
    }
    remove_file(cache_path).unwrap();
}

pub struct FileCacheSaver<T> {
    bound: usize,
    local: VecDeque<T>,
    sender: SyncSender<T>,
    retriever: Receiver<T>,
    join_handle: JoinHandle<()>,
}

impl<T> FileCacheSaver<T>
where
    T: Send + 'static + Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(bound: usize) -> Self {
        let (s1, r1) = sync_channel::<T>(bound);
        let (s2, r2) = sync_channel::<T>(bound);
        let join_handle = spawn(move || offload_proc(r1, s2));
        Self {
            bound,
            local: VecDeque::with_capacity(bound),
            sender: s1,
            retriever: r2,
            join_handle,
        }
    }

    pub fn into_retriever(self) -> FileCacheRetriever<T> {
        FileCacheRetriever {
            local: self.local,
            retriever: self.retriever,
            join_handle: Some(self.join_handle),
        }
    }

    pub fn push(&mut self, v: T) {
        if self.local.len() == self.bound {
            let v_to_send = self.local.pop_back().unwrap();
            self.sender.send(v_to_send).unwrap();
        }
        self.local.push_front(v);
    }
}

pub struct FileCacheRetriever<T> {
    local: VecDeque<T>,
    retriever: Receiver<T>,
    join_handle: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> FileCacheRetriever<T> {
    pub fn pop(&mut self) -> Option<T> {
        if self.local.is_empty() {
            self.retriever.recv().ok()
        } else {
            self.local.pop_front()
        }
    }
}

impl<T> Drop for FileCacheRetriever<T> {
    fn drop(&mut self) {
        self.join_handle.take().unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offload_test() {
        let mut reference = Vec::new();
        for i in 0..5 {
            reference.push(((i * 10)..((i + 1) * 10)).collect::<Vec<u64>>());
        }
        let mut cache = FileCacheSaver::new(2);
        for v in &reference {
            cache.push(v.to_owned());
        }
        let mut cache = cache.into_retriever();
        for v in reference.into_iter().rev() {
            assert_eq!(v, cache.pop().unwrap());
        }
    }
}
