use minimac::*;

const PORT: u16 = 8888;

fn main() {
    TcpCacheBackend::remote_proc(PORT, Some(6), OffloadCache::new(1000, FileCacheBackend));
}
