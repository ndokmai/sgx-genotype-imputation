use super::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::marker::PhantomData;

pub struct OffloadNtCache<B> {
    bound: usize,
    backend: B,
}

impl<B> OffloadNtCache<B> {
    pub fn new(bound: usize, backend: B) -> Self {
        Self { bound, backend }
    }
}

impl<B> Cache for OffloadNtCache<B>
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

pub struct OffloadNtCacheSave<T, B>
where
    B: CacheBackend,
{
    bound: usize,
    local: VecDeque<T>,
    backend: B::WriteBackend,
    _phantom: PhantomData<B>,
}

impl<T, B> OffloadNtCacheSave<T, B>
where
    T: Send + 'static,
    B: CacheBackend,
{
    pub fn new(bound: usize, backend: B::WriteBackend) -> Self {
        Self {
            bound,
            local: VecDeque::with_capacity(bound),
            backend,
            _phantom: PhantomData,
        }
    }
}

impl<T, B> CacheSave<T> for OffloadNtCacheSave<T, B>
where
    T: Send + 'static + Serialize + for<'de> Deserialize<'de>,
    B: CacheBackend + 'static,
    B::WriteBackend: Send + 'static,
    <B::WriteBackend as CacheWriteBackend>::ReadBackend: Send + 'static,
{
    type Load = OffloadNtCacheLoad<T, B>;

    fn push(&mut self, v: T) {
        if self.local.len() == self.bound {
            let v = self.local.pop_back().unwrap();
            self.backend.push_cache_item(&v).unwrap();
        }
        self.local.push_front(v);
    }

    fn into_load(self) -> Self::Load {
        OffloadNtCacheLoad {
            local: self.local,
            backend: self.backend.into_read(),
        }
    }
}

pub struct OffloadNtCacheLoad<T, B: CacheBackend> {
    local: VecDeque<T>,
    backend: <B::WriteBackend as CacheWriteBackend>::ReadBackend,
}

impl<T, B> CacheLoad<T> for OffloadNtCacheLoad<T, B>
where
    B: CacheBackend + 'static,
    T: Send + 'static + for<'de> Deserialize<'de>,
    <B::WriteBackend as CacheWriteBackend>::ReadBackend: Send + 'static,
{
    fn pop(&mut self) -> Option<T> {
        if self.local.is_empty() {
            self.backend.pop_cache_item().ok()
        } else {
            self.local.pop_front()
        }
    }
}
