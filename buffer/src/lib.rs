pub static PAGE_SIZE: usize = 8192; // 8Kb
pub static EVICTION_PERIOD: usize = 1; // 10 seconds
pub static BUFFER_CAPACITY: usize = 10; // 10 pages only within the pool
pub static FILES_METADATA: &str = "table-info";
pub static EVICTION_THREASHOLD: u32 = 0; // number of times the page can be found unused during eviction.

pub mod disk;
pub mod page;
pub mod pool;
pub mod request;
mod util;
