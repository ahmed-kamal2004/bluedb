use crate::ds::list::List;
use crate::error::BlueDbError;
use crate::error::BlueDbError::InternalError;
use crate::pool::flush::FlushList;
use crate::pool::pool_list::PoolList;
use crate::pool::wrpr::FrameWrpr;
use crate::pool::wrpr::ListType;
use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;
use std::sync::Mutex;
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
    pub page_hash: Arc<Mutex<std::collections::HashMap<String, Arc<FrameWrpr>>>>,
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
            page_hash: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Page_id layout
    /// Since BlueDB is Multi-file database, then the page_id translation must give out 2 things:
    /// - Which file we are targetting
    /// - Which page in this file
    ///
    /// In order to achieve that, we need to page_id to be containing basically 2 things
    /// {table_name}-{id}
    ///
    /// Where the table_name is translated using the catalog to the file_path
    /// While the id that starts from 1 is transalated to the specific offset as OFFSET = PAGE_SIZE * PAGE_ID
    pub fn get_page(&self, pg_id: &str) -> Result<Arc<FrameWrpr>, BlueDbError> {
        let hsh_tbl = self
            .page_hash
            .lock()
            .or_else(|_| Err(BlueDbError::InternalError))?; // blocking.

        // check if it exists in the table.
        if let Some(pg) = hsh_tbl.get(pg_id) {
            let mut pg = pg.clone();
            let is_front = unsafe { (*pg.prev.get()).is_null() };
            let is_back = unsafe { (*pg.next.get()).is_null() };
            match pg.lst_type {
                ListType::Upper => {
                    if !is_front && !is_back {
                        // remove it from this place and move to the start.
                        unsafe {
                            let frame = Arc::into_raw(pg) as *mut FrameWrpr;
                            (*frame)
                                .prev
                                .get()
                                .write((*frame).next.get() as *mut FrameWrpr);
                            pg = Arc::from_raw(frame);
                        }
                        self.new_sublist.push_front(pg); // TODO: fix the counter.
                    } else if is_front {
                        // Do nothing.
                    } else if is_back {
                        let page = self.new_sublist.pop_back();

                        match page {
                            Some(p) => {
                                self.new_sublist.push_back(p);
                            }
                            None => {
                                return Err(BlueDbError::PageLookupError);
                            }
                        }
                    }
                }
                ListType::Lower => {
                    if !is_front && !is_back {
                        // remove it from this place and move to the start.
                        unsafe {
                            let frame = Arc::into_raw(pg) as *mut FrameWrpr;
                            (*frame)
                                .prev
                                .get()
                                .write((*frame).next.get() as *mut FrameWrpr);
                            pg = Arc::from_raw(frame);
                        }
                        self.new_sublist.push_front(pg); // TODO: fix the counter.
                    } else if is_front {
                        let page = self.old_sublist.pop_front();

                        match page {
                            Some(p) => {
                                self.new_sublist.push_back(p);
                            }
                            None => {
                                return Err(BlueDbError::PageLookupError);
                            }
                        }
                    } else if is_back {
                        let page = self.old_sublist.pop_back();

                        match page {
                            Some(p) => {
                                self.new_sublist.push_back(p);
                            }
                            None => {
                                return Err(BlueDbError::PageLookupError);
                            }
                        }
                    }
                }
                ListType::None => {}
            }
        } else {
            // The page doesn't exist in the page table
            // it needs to be loaded from the disk.

            // the buffer pool needs to have a hash table that contains the file-descriptors.
        }

        Err(InternalError)
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
