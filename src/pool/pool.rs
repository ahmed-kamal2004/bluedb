use crate::pool::page::Page;
use std::rc::Weak;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU16;

pub struct FrameWrpr {
    /// the actual data
    pub frame: RwLock<Page>,
    /// for the linked list requirements.
    pub next: Weak<FrameWrpr>,
    pub prev: Weak<FrameWrpr>,
    /// checker for if the data is dirty or not.
    pub is_dirty: AtomicBool,
    /// counter for the current number of pins on the frame, if it is pinned, it cannot be evicted.
    pub is_pinned: AtomicU16,
    /// Immutable once created.
    /// Page ID is a combination of file name and page number, which is used to uniquely identify a page in the buffer pool.
    /// Example: "file1.txt:0" represents the first page of file1.txt, "file1.txt:1" represents the second page of file1.txt, and so on.
    pub page_id: String,
}

pub struct FrameList {
    pub head: Weak<FrameWrpr>,
    pub tail: Weak<FrameWrpr>,
}

/// InnoDB buffer pool implementation.
pub struct BufPl {
    pub db_path: String,

    pub old_sublist: Arc<RwLock<FrameList>>,
    pub old_sublist_capacity: usize,

    pub new_sublist: Arc<RwLock<FrameList>>,
    pub new_sublist_capacity: usize,

    pub free_list: Arc<RwLock<FrameList>>,
    pub free_list_capacity: usize,

    pub page_hash: Arc<RwLock<std::collections::HashMap<String, Arc<FrameWrpr>>>>,
}

// parsing the frame will be the responsibility of the caller (operator), the pool will only manage the frame's memory and lifecycle
// we will just need to have a way to get a specific frame from the pool, create a new frame to a specific file.

impl BufPl {
    pub fn new(db_path: String, free_list_capacity: usize) -> Self {
        let frame_lst = FrameList {
            head: Weak::new(),
            tail: Weak::new(),
        };

        BufPl {
            db_path,
            old_sublist: Arc::new(RwLock::new(FrameList {
                head: Weak::new(),
                tail: Weak::new(),
            })),
            old_sublist_capacity: 0,
            new_sublist: Arc::new(RwLock::new(FrameList {
                head: Weak::new(),
                tail: Weak::new(),
            })),
            new_sublist_capacity: 0,
            free_list: Arc::new(RwLock::new(FrameList {
                head: Weak::new(),
                tail: Weak::new(),
            })),
            free_list_capacity,
            page_hash: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}
