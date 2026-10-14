use std::sync::atomic::AtomicU64;
pub struct Statistics {
    pub total_pages_flushed: AtomicU64,
    pub total_flush_time: AtomicU64,
    pub total_pages_read: AtomicU64,
    pub total_read_time: AtomicU64,
    pub total_pages_written: AtomicU64,
    pub total_write_time: AtomicU64,

    pub total_pages_evicted: AtomicU64,
    pub total_eviction_time: AtomicU64,

    pub total_hits: AtomicU64,
    pub total_misses: AtomicU64,
    pub total_requests: AtomicU64,
}
