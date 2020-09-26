use super::*;
use crossbeam::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

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
    B: CacheBackend + 'static,
    B::WriteBackend: Send + 'static,
    <B::WriteBackend as CacheWriteBackend>::ReadBackend: Send + 'static,
{
    type Save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>> = OffloadCacheSave<T, B>;
    fn new_save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Self::Save<T> {
        Self::Save::new(self.bound, self.backend.new_write())
    }
}

pub struct OffloadCacheSave<T, B>
where
    B: CacheBackend,
{
    bound: usize,
    local: VecDeque<T>,
    send_queue: Sender<T>,
    send_dequeue: Receiver<T>,
    backend: Arc<Mutex<B::WriteBackend>>,
    _phantom: PhantomData<B>,
}

impl<T, B> OffloadCacheSave<T, B>
where
    T: Send + 'static,
    B: CacheBackend,
{
    pub fn new(bound: usize, backend: B::WriteBackend) -> Self {
        let (send_queue, send_dequeue) = bounded(bound);
        Self {
            bound,
            send_queue,
            send_dequeue,
            local: VecDeque::with_capacity(bound),
            backend: Arc::new(Mutex::new(backend)),
            _phantom: PhantomData,
        }
    }
}

impl<T, B> CacheSave<T> for OffloadCacheSave<T, B>
where
    T: Send + 'static + Serialize + for<'de> Deserialize<'de>,
    B: CacheBackend + 'static,
    B::WriteBackend: Send + 'static,
    <B::WriteBackend as CacheWriteBackend>::ReadBackend: Send + 'static,
{
    type Load = OffloadCacheLoad<T, B>;

    fn push(&mut self, v: T) {
        if self.local.len() == self.bound {
            let v_to_send = self.local.pop_back().unwrap();
            self.send_queue.send(v_to_send).unwrap();
            let backend = self.backend.clone();
            let send_dequeue = self.send_dequeue.clone();
            rayon::spawn(move || {
                let mut backend = backend.lock().unwrap();
                let v_to_send = send_dequeue.recv().unwrap();
                backend.push_cache_item(&v_to_send).unwrap();
            });
        }
        self.local.push_front(v);
    }

    fn into_load(self) -> Self::Load {
        let (send_cache, recv_cache) = bounded(self.bound);
        while Arc::strong_count(&self.backend) > 1 {}
        let backend = Arc::try_unwrap(self.backend)
            .ok()
            .unwrap()
            .into_inner()
            .unwrap()
            .into_read();
        let backend = Arc::new(Mutex::new(backend));
        OffloadCacheLoad::<_, B>::fill_buffer(send_cache.clone(), backend.clone());
        OffloadCacheLoad {
            local: self.local,
            backend,
            send_cache,
            recv_cache,
        }
    }
}

pub struct OffloadCacheLoad<T, B: CacheBackend> {
    local: VecDeque<T>,
    backend: Arc<Mutex<<B::WriteBackend as CacheWriteBackend>::ReadBackend>>,
    send_cache: Sender<Option<T>>,
    recv_cache: Receiver<Option<T>>,
}

impl<T, B> OffloadCacheLoad<T, B>
where
    T: Send + 'static + for<'de> Deserialize<'de>,
    B: CacheBackend,
    <B::WriteBackend as CacheWriteBackend>::ReadBackend: Send + 'static,
{
    fn fill_buffer(
        send_cache: Sender<Option<T>>,
        backend: Arc<Mutex<<B::WriteBackend as CacheWriteBackend>::ReadBackend>>,
    ) {
        if send_cache.is_full() {
            return;
        }
        rayon::spawn(move || {
            if let Ok(mut backend) = backend.lock() {
                for _ in 0..5 {
                    if send_cache.is_full() {
                        break;
                    }
                    if let Ok(v) = backend.pop_cache_item() {
                        if send_cache.send(Some(v)).is_err() {
                            break;
                        }
                    } else {
                        let _ = send_cache.send(None);
                        break;
                    }
                }
            }
        });
    }
}

impl<T, B> CacheLoad<T> for OffloadCacheLoad<T, B>
where
    B: CacheBackend + 'static,
    T: Send + 'static + for<'de> Deserialize<'de>,
    <B::WriteBackend as CacheWriteBackend>::ReadBackend: Send + 'static,
{
    fn pop(&mut self) -> Option<T> {
        if self.local.is_empty() {
            Self::fill_buffer(self.send_cache.clone(), self.backend.clone());
            self.recv_cache.recv().unwrap()
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
        let mut cache = OffloadCache::new(2, FileCacheBackend);
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
