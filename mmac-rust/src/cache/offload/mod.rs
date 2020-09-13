mod backend;
pub use backend::*;

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{Error, ErrorKind, Result};
use std::marker::PhantomData;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::{spawn, JoinHandle};

pub struct OffloadCache<B> {
    bound: usize,
    backend: B,
}

impl<B> OffloadCache<B> {
    pub fn new(bound: usize, backend: B) -> Self {
        Self { bound, backend }
    }
}

impl<B> Cache for OffloadCache<B>
where
    B: CacheBackend,
    B::WriteBackend: Send + 'static,
{
    type Save<T: Send + 'static> = OffloadCacheSave<T, B>;
    fn new_save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>(&self) -> Self::Save<T> {
        Self::Save::new(self.bound, self.backend.new_write())
    }
}

pub struct OffloadCacheSave<T, B>
where
    B: CacheBackend,
{
    bound: usize,
    local: VecDeque<T>,
    sender: SyncSender<T>,
    retriever: Receiver<T>,
    join_handle: JoinHandle<()>,
    _phantom: PhantomData<B>,
}

impl<T, B> OffloadCacheSave<T, B>
where
    T: Send + 'static + Serialize + for<'de> Deserialize<'de>,
    B: CacheBackend,
    B::WriteBackend: Send + 'static,
{
    pub fn new(bound: usize, cache_backend: B::WriteBackend) -> Self {
        let (s1, r1) = sync_channel::<T>(bound);
        let (s2, r2) = sync_channel::<T>(bound);
        let join_handle = spawn(move || Self::offload_proc(r1, s2, cache_backend).unwrap());
        Self {
            bound,
            local: VecDeque::with_capacity(bound),
            sender: s1,
            retriever: r2,
            join_handle,
            _phantom: PhantomData,
        }
    }

    fn offload_proc(
        r: Receiver<T>,
        s: SyncSender<T>,
        mut cache_write_backend: B::WriteBackend,
    ) -> Result<()> {
        loop {
            match r.recv() {
                Ok(v) => cache_write_backend.push_cache_item(&v)?,
                Err(_) => break,
            }
        }

        let mut cache_read_backend = cache_write_backend.into_read();
        loop {
            match cache_read_backend.pop_cache_item() {
                Ok(v) => s
                    .send(v)
                    .map_err(|_| Error::new(ErrorKind::Other, "Send error"))?,
                Err(_) => break,
            }
        }
        Ok(())
    }
}

impl<T, B> CacheSave<T> for OffloadCacheSave<T, B>
where
    T: Send + 'static,
    B: CacheBackend,
    B::WriteBackend: Send + 'static,
{
    type Load = OffloadCacheLoad<T>;

    fn push(&mut self, v: T) {
        if self.local.len() == self.bound {
            let v_to_send = self.local.pop_back().unwrap();
            self.sender.send(v_to_send).unwrap();
        }
        self.local.push_front(v);
    }

    fn into_load(self) -> Self::Load {
        OffloadCacheLoad {
            local: self.local,
            retriever: self.retriever,
            join_handle: Some(self.join_handle),
        }
    }
}

pub struct OffloadCacheLoad<T> {
    local: VecDeque<T>,
    retriever: Receiver<T>,
    join_handle: Option<JoinHandle<()>>,
}

impl<T> OffloadCacheLoad<T> {
    pub fn pop(&mut self) -> Option<T> {
        if self.local.is_empty() {
            self.retriever.recv().ok()
        } else {
            self.local.pop_front()
        }
    }
}

impl<T> Drop for OffloadCacheLoad<T> {
    fn drop(&mut self) {
        self.join_handle.take().unwrap().join().unwrap();
    }
}

impl<T> CacheLoad<T> for OffloadCacheLoad<T> {
    fn pop(&mut self) -> Option<T> {
        if self.local.is_empty() {
            self.retriever.recv().ok()
        } else {
            self.local.pop_front()
        }
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
        let cache = OffloadCache::new(2, FileCacheBackend);
        let mut save = cache.new_save();
        for v in &reference {
            save.push(v.to_owned());
        }
        let mut load = save.into_load();
        for v in reference.into_iter().rev() {
            assert_eq!(v, load.pop().unwrap());
        }
    }
}
