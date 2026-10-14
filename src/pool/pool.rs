use crate::ds::list::List;
use crate::page;
use crate::pool::flush::FlushList;
use crate::pool::pool_list::PoolList;
use crate::pool::wrpr::FrameWrpr;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicU32;

/// InnoDB buffer pool implementation.
pub struct BufPl {
    pub db_path: String,

    /// contains the 3/8 of the buffer pool for the one-time reads.
    pub old_sublist: Arc<List<PoolList>>,
    pub old_sublist_capacity: AtomicU32,

    /// contains the 5/8 of the buffer pool for the frequently accessed pages.
    pub new_sublist: Arc<List<PoolList>>,
    pub new_sublist_capacity: AtomicU32,

    /// Flush list contains the frames that are dirty and need to be flushed to disk. (Contains the same frames of the LRU list, but in a different order, sorted by the time of modification.)
    pub flush_list: Arc<FlushList>,

    /// contains the free frames that are not currently used.
    pub free_list: Arc<List<PoolList>>,

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

    pub fn get_free_frame(&self, page_id: &str) -> Option<Arc<FrameWrpr>> {
        // to obtain a free frame, we need to pop it from the free list, then add it the page hash and the LRU sublist.

        if let Some(mut frame) = self.free_list.pop_front() {
            unsafe {
                let mut frame_mut = Arc::into_raw(frame) as *mut FrameWrpr; // into_raw consumes the Arc.
                let frame_mut = &mut *frame_mut;
                frame_mut.set_page_id(page_id.to_string());
                frame = Arc::from_raw(frame_mut);
            }

            let mut hash_guard = self.page_hash.write().unwrap();

            hash_guard.insert(page_id.to_string(), frame.clone());

            drop(hash_guard);

            self.old_sublist.push_front(frame.clone());

            return Some(frame);
        }

        // no free frames available, we need to evict a frame from the LRU list.
        None
    }

    /*
    Thoughts

    Obtain a free block -> if not found, then evict a block from the LRU list.

    Eviction page must be
    - not dirty
    - not pinned

    - This scan will include the old sublist only, and if sizes are destroyed, it will rebalance the sizes of the two sublists.

    - The rebalancing is expensive, but it is amortized.

    - if there are two many dirty pages, then we will need to flush them to disk, and then evict them.
    (the flush in general should be managed by a background thread, but we can also do it in the foreground if needed.)


    - what about sequential pre reading ?

    - a page is moved from the old sublist to the new, when it is accessed again and it passed some time threshold.
     */
}
