use crate::pool::page::Page;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU16};
use std::sync::{Arc, RwLock};

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

    /// for the linked list requirements.
    pub next: UnsafeCell<*mut FrameWrpr>,
    pub prev: UnsafeCell<*mut FrameWrpr>,
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
            next: UnsafeCell::new(std::ptr::null_mut()),
            prev: UnsafeCell::new(std::ptr::null_mut()),
        }
    }

    pub fn new_empty() -> Self {
        FrameWrpr {
            frame: RwLock::new(Page::new()),
            is_dirty: AtomicBool::new(false),
            pin: AtomicU16::new(0),
            page_id: String::new(),
            next: UnsafeCell::new(std::ptr::null_mut()),
            prev: UnsafeCell::new(std::ptr::null_mut()),
        }
    }

    pub fn new_empty_with_page_id(page_id: String) -> Self {
        FrameWrpr {
            frame: RwLock::new(Page::new()),
            is_dirty: AtomicBool::new(false),
            pin: AtomicU16::new(0),
            page_id,
            next: UnsafeCell::new(std::ptr::null_mut()),
            prev: UnsafeCell::new(std::ptr::null_mut()),
        }
    }

    pub fn reinitialize_empty(&mut self) {
        self.frame = RwLock::new(Page::new());
        self.is_dirty
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.pin.store(0, std::sync::atomic::Ordering::SeqCst);
        self.page_id = String::new();
    }

    pub fn pin(&self) {
        self.pin.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn unpin(&self) {
        self.pin.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
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
