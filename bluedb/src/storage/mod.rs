// TODO: alignment
// TODO: start txn, commit, abort
// TODO: data directory metadata loading and storing
// TODO: slotted pages -> HEADER + SLOTS + DATA of the slots
// TODO: tuple data layout -> HEADER + Field (word aligned)
// TODO: tuple header -> Size

use buffer::pool::BufPool;
use buffer::builder::BufPoolBuilder;
use disk::disk::DiskManager;

pub struct StorageEngine {
    pool: BufPool,
}

impl StorageEngine {
    
}