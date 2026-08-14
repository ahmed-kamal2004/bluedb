use crate::pool::flush::FlushList;
use crate::pool::list::List;
use crate::pool::wrpr::FrameWrpr;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicU32;

/// InnoDB buffer pool implementation.
pub struct BufPl {
    pub db_path: String,

    /// contains the 3/8 of the buffer pool for the one-time reads.
    pub old_sublist: Arc<List>,
    pub old_sublist_capacity: AtomicU32,

    /// contains the 5/8 of the buffer pool for the frequently accessed pages.
    pub new_sublist: Arc<List>,
    pub new_sublist_capacity: AtomicU32,

    /// Flush list contains the frames that are dirty and need to be flushed to disk. (Contains the same frames of the LRU list, but in a different order, sorted by the time of modification.)
    pub flush_list: Arc<FlushList>,

    /// contains the free frames that are not currently used.
    pub free_list: Arc<List>,

    /// O(1) access/check for the frames in the buffer pool.
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
            flush_list: Arc::new(FlushList::new()),
            free_list,
            page_hash: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn get_frm(&self, pg_id: &str) -> Option<Arc<FrameWrpr>> {
        // first, check if the page already exists in the page hash.
        let page_hash = self.page_hash.read().unwrap();
        if let Some(frame) = page_hash.get(pg_id) {
            return Some(frame.clone());
        } else {
            // page doesn't exists in the hash.
            // we need to retrieve or create it.

            // first check if we have a free frame in the free list.
            let fr_lst_sz = self.free_list.size();
            if fr_lst_sz > 0 {}

            None
        }
    }
}
