use super::*;
use std::io::{Error, ErrorKind, Result};
use std::os::fortanix_sgx::usercalls::raw::ByteBuffer;
use std::os::fortanix_sgx::usercalls::{alloc, free};
use std::ptr::copy;

struct UnsafeWrapper(ByteBuffer);
unsafe impl Send for UnsafeWrapper {}

pub struct NonEnclaveLocalCacheBackend;

impl CacheBackend for NonEnclaveLocalCacheBackend {
    type WriteBackend = NonEnclaveLocalCacheWriteBackend;
    fn new_write(&mut self) -> Self::WriteBackend {
        NonEnclaveLocalCacheWriteBackend(Vec::new())
    }
}

pub struct NonEnclaveLocalCacheWriteBackend(Vec<UnsafeWrapper>);

impl CacheWriteBackend for NonEnclaveLocalCacheWriteBackend {
    type ReadBackend = NonEnclaveLocalCacheReadBackend;

    fn into_read(self) -> Self::ReadBackend {
        NonEnclaveLocalCacheReadBackend(self.0.into_iter().rev())
    }

    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()> {
        let buffer = bincode::serialize(v).unwrap();
        let ptr = alloc(buffer.len(), 1)?;
        unsafe { copy(buffer.as_ptr(), ptr, buffer.len()) };
        let buffer = ByteBuffer {
            data: ptr,
            len: buffer.len(),
        };
        self.0.push(UnsafeWrapper(buffer));
        Ok(())
    }
}

pub struct NonEnclaveLocalCacheReadBackend(std::iter::Rev<std::vec::IntoIter<UnsafeWrapper>>);

impl CacheReadBackend for NonEnclaveLocalCacheReadBackend {
    fn pop_cache_item<T: for<'de> serde::Deserialize<'de>>(&mut self) -> Result<T> {
        let buffer = self
            .0
            .next()
            .ok_or(Error::new(ErrorKind::Other, "Out of cache items"))?
            .0;
        let (ptr, len) = (buffer.data as *mut u8, buffer.len);
        let buffer = unsafe { std::slice::from_raw_parts(ptr, len) };
        let out = bincode::deserialize(buffer).map_err(|e| Error::new(ErrorKind::Other, e))?;
        unsafe { free(ptr, len, 1) };
        Ok(out)
    }
}
