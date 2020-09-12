pub struct LocalCacheSaver<T>(Vec<T>);

impl<T> LocalCacheSaver<T> {
    pub fn new(_: usize) -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub fn push(&mut self, v: T) {
        self.0.push(v)
    }

    pub fn into_retriever(self) -> LocalCacheRetriever<T> {
        LocalCacheRetriever(self.0)
    }
}

pub struct LocalCacheRetriever<T>(Vec<T>);

impl<T> LocalCacheRetriever<T> {
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }
}
