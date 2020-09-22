use super::*;

pub struct LocalCache;

impl Cache for LocalCache {
    type Save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>> = LocalCacheSave<T>;
    fn new_save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Self::Save<T> {
        LocalCacheSave::new()
    }
}

pub struct LocalCacheSave<T>(Vec<T>);

impl<T> LocalCacheSave<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<T: Send + 'static + Serialize + for<'de> Deserialize<'de>> CacheSave<T> for LocalCacheSave<T> {
    type Load = LocalCacheLoad<T>;
    #[inline]
    fn push(&mut self, v: T) {
        self.0.push(v)
    }

    fn into_load(self) -> Self::Load {
        LocalCacheLoad(self.0)
    }
}

pub struct LocalCacheLoad<T>(Vec<T>);

impl<T: Send + for<'de> Deserialize<'de>> CacheLoad<T> for LocalCacheLoad<T> {
    #[inline]
    fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }
}
