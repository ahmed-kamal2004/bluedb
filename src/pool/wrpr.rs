use crate::pool::page::Page;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU16};
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameListType {
    Old,
    New,
    Free,
}

pub struct FrameWrpr {
    /// the actual data
    pub frame: RwLock<Page>,
    /// checker for if the data is dirty or not.
    pub is_dirty: AtomicBool,
    /// counter for the current number of pins on the frame, if it is pinned, it cannot be evicted.
    pub pin: AtomicU16,
    /// Immutable once created.
    /// Page ID is a combination of file name and page number, which is used to uniquely identify a page in the buffer pool.
    /// Example: "file1.txt:0" represents the first page of file1.txt, "file1.txt:1" represents the second page of file1.txt, and so on.
    pub page_id: String,
    /// Inner details.
    pub inner: RwLock<FrameWrprInner>,

    /// for the linked list requirements. (LRU list)
    pub next: UnsafeCell<*mut FrameWrpr>,
    pub prev: UnsafeCell<*mut FrameWrpr>,

    /// for the linked list requirements. (Flush list)
    pub oldest_modification_data: usize,
    pub next_flush: UnsafeCell<*mut FrameWrpr>,
}

pub struct FrameWrprInner {
    /// Last time the page was accessed.
    pub last_accessed: std::time::Instant,
    /// for the linked list requirements.
    pub lst_tp: FrameListType,
}

// hey, Compiler, I know what I'm doing, trust me.
unsafe impl Send for FrameWrpr {}
unsafe impl Sync for FrameWrpr {}

impl FrameWrpr {
    pub fn new_from_page(page_id: String, frame: Page) -> Self {
        FrameWrpr {
            frame: RwLock::new(frame),
            is_dirty: AtomicBool::new(false),
            pin: AtomicU16::new(0),
            page_id,
            inner: RwLock::new(FrameWrprInner {
                last_accessed: std::time::Instant::now(),
                lst_tp: FrameListType::Free,
            }),
            next: UnsafeCell::new(std::ptr::null_mut()),
            prev: UnsafeCell::new(std::ptr::null_mut()),
            oldest_modification_data: 0,
            next_flush: UnsafeCell::new(std::ptr::null_mut()),
        }
    }

    pub fn new_empty() -> Self {
        FrameWrpr {
            frame: RwLock::new(Page::new()),
            is_dirty: AtomicBool::new(false),
            pin: AtomicU16::new(0),
            page_id: String::new(),
            inner: RwLock::new(FrameWrprInner {
                last_accessed: std::time::Instant::now(),
                lst_tp: FrameListType::Free,
            }),
            next: UnsafeCell::new(std::ptr::null_mut()),
            prev: UnsafeCell::new(std::ptr::null_mut()),
            oldest_modification_data: 0,
            next_flush: UnsafeCell::new(std::ptr::null_mut()),
        }
    }

    pub fn new_empty_with_page_id(page_id: String) -> Self {
        FrameWrpr {
            frame: RwLock::new(Page::new()),
            is_dirty: AtomicBool::new(false),
            pin: AtomicU16::new(0),
            page_id,
            inner: RwLock::new(FrameWrprInner {
                last_accessed: std::time::Instant::now(),
                lst_tp: FrameListType::Free,
            }),
            next: UnsafeCell::new(std::ptr::null_mut()),
            prev: UnsafeCell::new(std::ptr::null_mut()),
            oldest_modification_data: 0,
            next_flush: UnsafeCell::new(std::ptr::null_mut()),
        }
    }

    pub fn reinitialize_empty(&mut self) {
        self.frame = RwLock::new(Page::new());
        self.is_dirty
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.pin.store(0, std::sync::atomic::Ordering::SeqCst);
        self.page_id = String::new();
        self.inner = RwLock::new(FrameWrprInner {
            last_accessed: std::time::Instant::now(),
            lst_tp: FrameListType::Free,
        });
    }

    pub fn pin(&self) {
        self.pin.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn unpin(&self) {
        self.pin.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_pinned(&self) -> bool {
        self.pin.load(std::sync::atomic::Ordering::SeqCst) > 0
    }

    pub fn update_last_accessed(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.last_accessed = std::time::Instant::now();
    }

    pub fn last_accessed_duration_cmp(&self, other: &Duration) -> std::cmp::Ordering {
        let last_accessed = self.inner.read().unwrap().last_accessed;
        let duration_since_last_accessed = last_accessed.elapsed();
        duration_since_last_accessed.cmp(other)
    }

    pub fn get_frame_list_type(&self) -> FrameListType {
        self.inner.read().unwrap().lst_tp.clone()
    }

    pub fn update_frame_list_type(&self, lst_tp: FrameListType) {
        let mut inner = self.inner.write().unwrap();
        inner.lst_tp = lst_tp;
    }

    pub fn make_dirty(&self) {
        self.is_dirty
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_page(&self) -> &RwLock<Page> {
        &self.frame
    }

    pub fn get_page_id(&self) -> &String {
        &self.page_id
    }

    pub fn set_page_id(&mut self, page_id: String) {
        self.page_id = page_id;
    }
}
