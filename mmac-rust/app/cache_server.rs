use mmac::{FileCacheBackend, OffloadCache, TcpCacheBackend};

const PORT: u16 = 8888;

fn main() {
    TcpCacheBackend::remote_proc(PORT, OffloadCache::new(500, FileCacheBackend));
}
