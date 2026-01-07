pub static PAGE_SIZE: usize = 8192; // 8Kb

pub mod pool;
pub(crate) mod disk;
pub(crate) mod page;
pub(crate) mod request;
pub(crate) mod utils;