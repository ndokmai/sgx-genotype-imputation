mod backend;
mod local;
#[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
mod nonenclave;
mod offload;
mod offload_nt;

pub use backend::*;
pub use local::*;
#[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
pub use nonenclave::*;
pub use offload::*;
pub use offload_nt::*;

use serde::{Deserialize, Serialize};

pub trait Cache {
    type Save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>: CacheSave<T>;
    fn new_save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Self::Save<T>;
}

pub trait CacheSave<T: Send + 'static + Serialize + for<'de> Deserialize<'de>> {
    type Load: CacheLoad<T>;
    fn push(&mut self, v: T);
    fn into_load(self) -> Self::Load;
}

pub trait CacheLoad<T: Send + for<'de> Deserialize<'de>> {
    fn pop(&mut self) -> Option<T>;
}
