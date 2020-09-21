use crate::{
    tcp_keep_connecting, Cache, CacheBackend, CacheLoad, CacheReadBackend, CacheSave,
    CacheWriteBackend,
};
use bufstream::BufStream;
use byteorder::{ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind, Result, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

pub struct TcpCacheBackend {
    addr: SocketAddr,
    capacities: Option<(usize, usize)>,
}

impl TcpCacheBackend {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            capacities: None,
        }
    }

    pub fn with_capacities(reader_cap: usize, writer_cap: usize, addr: SocketAddr) -> Self {
        Self {
            addr,
            capacities: Some((reader_cap, writer_cap)),
        }
    }

    pub fn remote_proc<B>(port: u16, n_caches: Option<usize>, mut cache_backend: B)
    where
        B: Cache,
        B::Save<Vec<u8>>: Send + 'static,
    {
        let listener = TcpListener::bind(("localhost", port)).unwrap();
        let mut handles = Vec::new();
        for (i, stream) in listener.incoming().enumerate() {
            let mut stream = BufStream::new(stream.unwrap());
            let mut cache_save = cache_backend.new_save();
            handles.push(std::thread::spawn(move || {
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
            }));
            if let Some(n_caches) = n_caches {
                if i == n_caches - 1 {
                    break;
                }
            }
        }
        for handle in handles.into_iter() {
            handle.join().unwrap();
        }
    }
}

impl CacheBackend for TcpCacheBackend {
    type WriteBackend = TcpCacheWriteBackend;
    fn new_write(&mut self) -> Self::WriteBackend {
        let stream = tcp_keep_connecting(self.addr);
        let stream = match self.capacities {
            Some((reader_cap, writer_cap)) => {
                BufStream::with_capacities(reader_cap, writer_cap, stream)
            }
            None => BufStream::new(stream),
        };
        TcpCacheWriteBackend { stream }
    }
}

pub struct TcpCacheWriteBackend {
    stream: BufStream<TcpStream>,
}

impl CacheWriteBackend for TcpCacheWriteBackend {
    type ReadBackend = TcpCacheReadBackend;
    fn into_read(mut self) -> Self::ReadBackend {
        self.stream.write_u8(Cmd::Pop.into()).unwrap();
        self.stream.flush().unwrap();

        TcpCacheReadBackend(self.stream)
    }

    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()> {
        self.stream.write_u8(Cmd::Push.into())?;
        let buffer = bincode::serialize(v).unwrap();
        bincode::serialize_into(&mut self.stream, &buffer)
            .map_err(|e| Error::new(ErrorKind::Other, e))
    }
}

pub struct TcpCacheReadBackend(BufStream<TcpStream>);

impl CacheReadBackend for TcpCacheReadBackend {
    fn pop_cache_item<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        let buffer: Vec<u8> =
            bincode::deserialize_from(&mut self.0).map_err(|e| Error::new(ErrorKind::Other, e))?;
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
            TcpCacheBackend::remote_proc(port, Some(1), LocalCache);
        });

        let mut reference = Vec::new();
        for i in 0..5 {
            reference.push(((i * 10)..((i + 1) * 10)).collect::<Vec<u64>>());
        }
        let mut cache = TcpCacheBackend::new(addr);
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
