use super::*;
use std::marker::PhantomData;
use std::os::fortanix_sgx::usercalls::raw::ByteBuffer;
use std::os::fortanix_sgx::usercalls::{alloc, free};
use std::ptr::copy;

struct UnsafeWrapper(ByteBuffer);
unsafe impl Send for UnsafeWrapper {}

pub struct NonenclaveCache;

impl Cache for NonenclaveCache {
    type Save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>> = NonenclaveCacheSave<T>;
    fn new_save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Self::Save<T> {
        NonenclaveCacheSave {
            inner: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

pub struct NonenclaveCacheSave<T> {
    inner: Vec<UnsafeWrapper>,
    _phantom: PhantomData<T>,
}

impl<T> CacheSave<T> for NonenclaveCacheSave<T>
where
    T: Send + 'static + Serialize + for<'de> Deserialize<'de>,
{
    type Load = NonenclaveCacheLoad<T>;
    fn push(&mut self, v: T) {
        let buffer = bincode::serialize(&v).unwrap();
        let ptr = alloc(buffer.len(), 1).unwrap();
        unsafe { copy(buffer.as_ptr(), ptr, buffer.len()) };
        let buffer = ByteBuffer {
            data: ptr,
            len: buffer.len(),
        };
        self.inner.push(UnsafeWrapper(buffer));
    }

    fn into_load(self) -> Self::Load {
        NonenclaveCacheLoad {
            inner: self.inner,
            _phantom: PhantomData,
        }
    }
}

pub struct NonenclaveCacheLoad<T> {
    inner: Vec<UnsafeWrapper>,
    _phantom: PhantomData<T>,
}

impl<T> CacheLoad<T> for NonenclaveCacheLoad<T>
where
    T: Send + for<'de> Deserialize<'de>,
{
    fn pop(&mut self) -> Option<T> {
        let buffer = self.inner.pop()?.0;
        let (ptr, len) = (buffer.data as *mut u8, buffer.len);
        let buffer = unsafe { std::slice::from_raw_parts(ptr, len) };
        let out = bincode::deserialize(buffer).unwrap();
        unsafe { free(ptr, len, 1) };
        Some(out)
    }
}
