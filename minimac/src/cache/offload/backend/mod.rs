mod encryption;
mod file;
#[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
mod nonenclave_local;
mod tcp;

pub use encryption::*;
pub use file::*;
#[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
pub use nonenclave_local::*;
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
