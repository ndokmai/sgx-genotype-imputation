use mmac::*;

const PORT: u16 = 8888;

fn main() {
    TcpCacheBackend::remote_proc(PORT, OffloadCache::new(1000, FileCacheBackend));
}
