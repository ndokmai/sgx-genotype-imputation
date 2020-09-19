#![allow(dead_code)]
mod local;
mod offload;

pub use local::*;
pub use offload::*;
use serde::{Deserialize, Serialize};

pub trait Cache {
    type Save<T: Send + 'static>: CacheSave<T>;
    fn new_save<T: Send + 'static + Serialize + for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Self::Save<T>;
}

pub trait CacheSave<T> {
    type Load: CacheLoad<T>;
    fn push(&mut self, v: T);
    fn into_load(self) -> Self::Load;
}

pub trait CacheLoad<T> {
    fn pop(&mut self) -> Option<T>;
}
