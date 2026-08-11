use crate::pool::list::List;
use crate::pool::wrpr::FrameWrpr;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicU32;

/// InnoDB buffer pool implementation.
pub struct BufPl {
    pub db_path: String,

    pub old_sublist: Arc<List>,
    pub old_sublist_capacity: AtomicU32,

    pub new_sublist: Arc<List>,
    pub new_sublist_capacity: AtomicU32,

    pub free_list: Arc<List>,
    pub free_list_capacity: AtomicU32,

    pub page_hash: Arc<RwLock<std::collections::HashMap<String, Arc<FrameWrpr>>>>,
}

// parsing the frame will be the responsibility of the caller (operator), the pool will only manage the frame's memory and lifecycle
// we will just need to have a way to get a specific frame from the pool, create a new frame to a specific file.

impl BufPl {
    pub fn new(db_path: String, free_list_capacity: usize) -> Self {
        let free_list = Arc::new(List::new());

        for _ in 0..free_list_capacity {
            let frame = Arc::new(FrameWrpr::new_empty());
            free_list.push_back(frame);
        }

        BufPl {
            db_path,
            old_sublist: Arc::new(List::new()),
            old_sublist_capacity: AtomicU32::new(0),
            new_sublist: Arc::new(List::new()),
            new_sublist_capacity: AtomicU32::new(0),
            free_list,
            free_list_capacity: AtomicU32::new(free_list_capacity as u32),
            page_hash: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}
