use crate::{
    tcp_keep_connecting, Cache, CacheBackend, CacheLoad, CacheReadBackend, CacheSave,
    CacheWriteBackend,
};
use bufstream::BufStream;
use byteorder::{ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind, Result, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{sync_channel, Receiver};
use std::thread::spawn;

pub struct TcpCacheBackend(SocketAddr);

impl TcpCacheBackend {
    pub fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }

    pub fn remote_proc<B>(port: u16, mut cache_backend: B)
    where
        B: Cache,
        B::Save<Vec<u8>>: Send + 'static,
    {
        let listener = TcpListener::bind(("localhost", port)).unwrap();
        for stream in listener.incoming() {
            let mut stream = BufStream::new(stream.unwrap());
            let mut cache_save = cache_backend.new_save();
            spawn(move || {
                loop {
                    let cmd: Cmd = stream.read_u8().unwrap().into();
                    match cmd {
                        Cmd::Push => {
                            let buffer: Vec<u8> = bincode::deserialize_from(&mut stream).unwrap();
                            cache_save.push(buffer);
                        }
                        Cmd::Pop => break,
                    }
                }
                let mut cache_load = cache_save.into_load();
                loop {
                    if let Some(cache_item) = cache_load.pop() {
                        bincode::serialize_into(&mut stream, &cache_item).unwrap();
                    } else {
                        break;
                    }
                }
            });
        }
    }
}

impl CacheBackend for TcpCacheBackend {
    type WriteBackend = TcpCacheWriteBackend;
    fn new_write(&mut self) -> Self::WriteBackend {
        TcpCacheWriteBackend(BufStream::new(tcp_keep_connecting(self.0)))
    }
}

pub struct TcpCacheWriteBackend(BufStream<TcpStream>);

impl CacheWriteBackend for TcpCacheWriteBackend {
    type ReadBackend = TcpCacheReadBackend;
    fn into_read(mut self) -> Self::ReadBackend {
        self.0.write_u8(Cmd::Pop.into()).unwrap();
        self.0.flush().unwrap();

        let (s, r) = sync_channel(20);

        spawn(move || {
            let mut stream = self.0;
            loop {
                if let Ok(buffer) = bincode::deserialize_from(&mut stream) {
                    s.send(buffer).unwrap();
                } else {
                    break;
                }
            }
        });
        TcpCacheReadBackend(r)
    }

    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()> {
        self.0.write_u8(Cmd::Push.into())?;
        let buffer = bincode::serialize(v).unwrap();
        bincode::serialize_into(&mut self.0, &buffer).map_err(|e| Error::new(ErrorKind::Other, e))
    }
}

pub struct TcpCacheReadBackend(Receiver<Vec<u8>>);

impl CacheReadBackend for TcpCacheReadBackend {
    fn pop_cache_item<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        let buffer = self.0.recv().map_err(|e| Error::new(ErrorKind::Other, e))?;
        Ok(bincode::deserialize_from(&buffer[..]).unwrap())
    }
}

enum Cmd {
    Push,
    Pop,
}

impl From<u8> for Cmd {
    fn from(code: u8) -> Self {
        match code {
            0 => Self::Push,
            1 => Self::Pop,
            _ => panic!("Invalid code"),
        }
    }
}

impl Into<u8> for Cmd {
    fn into(self) -> u8 {
        match self {
            Self::Push => 0,
            Self::Pop => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalCache;

    #[test]
    fn tcp_test() {
        let port: u16 = 8888;
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();

        std::thread::spawn(move || {
            TcpCacheBackend::remote_proc(port, LocalCache);
        });

        let mut reference = Vec::new();
        for i in 0..5 {
            reference.push(((i * 10)..((i + 1) * 10)).collect::<Vec<u64>>());
        }
        let mut cache = TcpCacheBackend(addr);
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