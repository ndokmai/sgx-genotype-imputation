mod file;
pub use file::*;

use serde::{Deserialize, Serialize};
use std::io::Result;
pub trait CacheBackend {
    type WriteBackend: CacheWriteBackend;
    fn new_write(&self) -> Self::WriteBackend;
}

pub trait CacheWriteBackend {
    type ReadBackend: CacheReadBackend;
    fn into_read(self) -> Self::ReadBackend;
    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()>;
}

pub trait CacheReadBackend {
    fn pop_cache_item<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T>;
}
