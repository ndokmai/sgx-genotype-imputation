mod encryption;
mod file;
mod tcp;
pub use encryption::*;
pub use file::*;
pub use tcp::*;

use serde::{Deserialize, Serialize};
use std::io::Result;
pub trait CacheBackend {
    type WriteBackend: CacheWriteBackend;
    fn new_write(&mut self) -> Self::WriteBackend;
}

pub trait CacheWriteBackend {
    type ReadBackend: CacheReadBackend;
    fn into_read(self) -> Self::ReadBackend;
    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()>;
}

pub trait CacheReadBackend {
    fn pop_cache_item<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T>;
}
