use super::{CacheBackend, CacheReadBackend, CacheWriteBackend};
use serde::{Deserialize, Serialize};
use std::fs::{remove_file, File};
use std::io::{BufReader, BufWriter, Error, ErrorKind, Result, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

static CACHEFILE_ROOT: &str = "/tmp";

pub struct FileCacheBackend;

impl CacheBackend for FileCacheBackend {
    type WriteBackend = FileCacheWriteBackend;
    fn new_write(&self) -> Self::WriteBackend {
        FileCacheWriteBackend::new()
    }
}

pub struct FileCacheWriteBackend {
    path: PathBuf,
    bytes_positions: Vec<u64>,
    file: BufWriter<File>,
}

impl FileCacheWriteBackend {
    pub fn new() -> Self {
        let filename = format!("{}.cache", rand::random::<u64>());
        let path = Path::new(CACHEFILE_ROOT).join(filename).to_owned();
        let file = BufWriter::new(File::create(&path).unwrap());
        let bytes_positions = Vec::new();
        Self {
            path,
            bytes_positions,
            file,
        }
    }

    pub fn handle_error(&self) {
        let _ = remove_file(&self.path);
    }
}

impl CacheWriteBackend for FileCacheWriteBackend {
    type ReadBackend = FileCacheReadBackend;
    fn into_read(mut self) -> Self::ReadBackend {
        self.file
            .flush()
            .map_err(|e| {
                self.handle_error();
                e
            })
            .unwrap();
        let file = BufReader::new(File::open(&self.path).unwrap());
        FileCacheReadBackend {
            path: self.path,
            bytes_positions: self.bytes_positions,
            file,
        }
    }

    fn push_cache_item<T: Serialize>(&mut self, v: &T) -> Result<()> {
        self.bytes_positions
            .push(self.file.stream_position().map_err(|e| {
                self.handle_error();
                e
            })?);
        bincode::serialize_into(&mut self.file, v).map_err(|e| {
            self.handle_error();
            Error::new(ErrorKind::Other, e)
        })
    }
}

pub struct FileCacheReadBackend {
    path: PathBuf,
    bytes_positions: Vec<u64>,
    file: BufReader<File>,
}

impl FileCacheReadBackend {
    pub fn handle_error(&self) {
        let _ = remove_file(&self.path);
    }
}

impl Drop for FileCacheReadBackend {
    fn drop(&mut self) {
        let _ = remove_file(&self.path);
    }
}

impl CacheReadBackend for FileCacheReadBackend {
    fn pop_cache_item<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        let pos = self.bytes_positions.pop().ok_or_else(|| {
            self.handle_error();
            Error::new(ErrorKind::UnexpectedEof, "Out of cache items")
        })?;
        self.file.seek(SeekFrom::Start(pos)).map_err(|e| {
            self.handle_error();
            e
        })?;
        bincode::deserialize_from(&mut self.file).map_err(|e| {
            self.handle_error();
            Error::new(ErrorKind::Other, e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_test() {
        let mut reference = Vec::new();
        for i in 0..5 {
            reference.push(((i * 10)..((i + 1) * 10)).collect::<Vec<u64>>());
        }
        let cache = FileCacheBackend;
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
