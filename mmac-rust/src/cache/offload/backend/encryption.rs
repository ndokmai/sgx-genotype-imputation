use crate::{CacheBackend, CacheReadBackend, CacheWriteBackend};
use aes_gcm::aead::generic_array::{typenum::Unsigned, GenericArray};
use aes_gcm::{AeadInPlace, Aes128Gcm, NewAead};
use byteorder::{NetworkEndian, WriteBytesExt};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind, Result};

type Cipher = Aes128Gcm;
type KeySize = <Cipher as NewAead>::KeySize;
type NonceSize = <Cipher as AeadInPlace>::NonceSize;
type CiphertextOverhead = <Cipher as AeadInPlace>::CiphertextOverhead;
type Nonce = GenericArray<u8, NonceSize>;
type Key = GenericArray<u8, KeySize>;

pub struct EncryptedCacheBackend<B> {
    cipher: Cipher,
    backend: B,
}

impl<B: CacheBackend> EncryptedCacheBackend<B> {
    pub fn new(backend: B) -> Self {
        let mut key = Key::default();
        thread_rng().fill(key.as_mut_slice());
        Self {
            cipher: Cipher::new(&key),
            backend,
        }
    }
}

impl<B: CacheBackend> CacheBackend for EncryptedCacheBackend<B> {
    type WriteBackend = EncryptedCacheWriteBackend<B::WriteBackend>;
    fn new_write(&self) -> Self::WriteBackend {
        EncryptedCacheWriteBackend {
            counter: 0,
            cipher: self.cipher.clone(),
            backend: self.backend.new_write(),
        }
    }
}

pub struct EncryptedCacheWriteBackend<B> {
    counter: u32,
    cipher: Cipher,
    backend: B,
}

impl<B: CacheWriteBackend> CacheWriteBackend for EncryptedCacheWriteBackend<B> {
    type ReadBackend = EncryptedCacheReadBackend<B::ReadBackend>;
    fn into_read(self) -> Self::ReadBackend {
        EncryptedCacheReadBackend {
            countdown: self.counter,
            cipher: self.cipher,
            backend: self.backend.into_read(),
        }
    }

    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()> {
        let mut ciphertext: Vec<u8> =
            bincode::serialize(&v).map_err(|e| Error::new(ErrorKind::Other, e))?;
        ciphertext.resize(ciphertext.len() + CiphertextOverhead::USIZE, 0);
        let mut nonce = Nonce::default();
        thread_rng().fill(nonce.as_mut_slice());
        let mut counter_buf: Vec<u8> = Vec::new();
        counter_buf.write_u32::<NetworkEndian>(self.counter as u32)?;
        self.counter += 1;
        self.cipher
            .encrypt_in_place(&nonce, &counter_buf[..], &mut ciphertext)
            .map_err(|_| Error::new(ErrorKind::Other, "aead::Error"))?;
        self.backend.push_cache_item(&(nonce, ciphertext))
    }
}

pub struct EncryptedCacheReadBackend<B> {
    countdown: u32,
    cipher: Cipher,
    backend: B,
}

impl<B: CacheReadBackend> CacheReadBackend for EncryptedCacheReadBackend<B> {
    fn pop_cache_item<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        self.countdown -= 1;
        let (nonce, mut ciphertext): (Nonce, Vec<u8>) = self.backend.pop_cache_item()?;
        let mut counter_buf: Vec<u8> = Vec::new();
        counter_buf.write_u32::<NetworkEndian>(self.countdown as u32)?;
        self.cipher
            .decrypt_in_place(&nonce, &counter_buf[..], &mut ciphertext)
            .map_err(|_| Error::new(ErrorKind::Other, "aead::Error"))?;
        bincode::deserialize(&ciphertext[..]).map_err(|e| Error::new(ErrorKind::Other, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileCacheBackend;

    #[test]
    fn encrypted_file_test() {
        let mut reference = Vec::new();
        for i in 0..5 {
            reference.push(((i * 10)..((i + 1) * 10)).collect::<Vec<u64>>());
        }
        let cache = EncryptedCacheBackend::new(FileCacheBackend);
        let mut file = cache.new_write();
        for v in &reference {
            file.push_cache_item(v).unwrap();
        }
        let mut file = file.into_read();
        for v in reference.into_iter().rev() {
            let cached_item: Vec<u64> = file.pop_cache_item().unwrap();
            assert_eq!(v, cached_item);
        }
    }
}
