use super::super::ds::list_state::ListState;
use crate::{ds::list_state::HasDestructor, pool::wrpr::FrameWrpr};
use std::sync::Arc;
pub struct PoolList {
    head: *mut FrameWrpr,
    tail: *mut FrameWrpr,
}

impl Drop for PoolList {
    fn drop(&mut self) {
        while let Some(frame) = self.pop_front() {
            drop(frame);
        }
    }
}

impl HasDestructor for PoolList {} // the impl of `HasDestructor` means that Drop is implemented as a code-convention

impl ListState for PoolList {
    type Item = Arc<FrameWrpr>;
    type RawItem = *mut FrameWrpr;

    fn new() -> Self {
        PoolList {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
        }
    }

    fn push_front(&mut self, item: Self::Item) {
        let frame_ptr = Arc::into_raw(item) as *mut FrameWrpr;

        unsafe {
            (*frame_ptr).next.get().write(self.head);
            (*frame_ptr).prev.get().write(std::ptr::null_mut());
            if !self.head.is_null() {
                (*self.head).prev.get().write(frame_ptr);
            }
            self.head = frame_ptr;
            if self.tail.is_null() {
                self.tail = frame_ptr;
            }
        }
    }

    fn push_back(&mut self, item: Self::Item) {
        let frame_ptr = Arc::into_raw(item) as *mut FrameWrpr;

        unsafe {
            (*frame_ptr).prev.get().write(self.tail);
            (*frame_ptr).next.get().write(std::ptr::null_mut());
            if !self.tail.is_null() {
                (*self.tail).next.get().write(frame_ptr);
            }
            self.tail = frame_ptr;
            if self.head.is_null() {
                self.head = frame_ptr;
            }
        }
    }

    fn pop_front(&mut self) -> Option<Self::Item> {
        if self.head.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = self.head;
            self.head = *(*frame_ptr).next.get();
            if !self.head.is_null() {
                (*self.head).prev.get().write(std::ptr::null_mut());
            } else {
                self.tail = std::ptr::null_mut();
            }
            Some(Arc::from_raw(frame_ptr))
        }
    }

    fn pop_back(&mut self) -> Option<Self::Item> {
        if self.tail.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = self.tail;
            self.tail = *(*frame_ptr).prev.get();
            if !self.tail.is_null() {
                (*self.tail).next.get().write(std::ptr::null_mut());
            } else {
                self.head = std::ptr::null_mut();
            }
            Some(Arc::from_raw(frame_ptr))
        }
    }

    fn head(&self) -> Self::RawItem {
        self.head
    }

    fn next(&self, curr: &mut Self::RawItem) -> Option<Self::Item> {
        if curr.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = curr.as_ref()?;
            *curr = *(frame_ptr.next.get());
            let frame_arc = Arc::from_raw(frame_ptr);
            let temp_arc = Arc::clone(&frame_arc);
            let _ = Arc::into_raw(frame_arc); // Prevent dropping the original Arc (doesn't decrement the ref count)
            Some(temp_arc)
        }
    }
}

unsafe impl Send for PoolList {}
unsafe impl Sync for PoolList {}
