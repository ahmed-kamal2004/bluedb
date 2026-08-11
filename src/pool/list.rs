use crate::pool::page::Page;
use crate::pool::wrpr::FrameWrpr;
use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU16;

pub struct List {
    inner: Mutex<State>,
}

pub struct State {
    head: *mut FrameWrpr,
    tail: *mut FrameWrpr,
    size: usize,
}

unsafe impl Send for List {}
unsafe impl Sync for List {}

impl List {
    pub fn new() -> Self {
        List {
            inner: Mutex::new(State {
                head: std::ptr::null_mut(),
                tail: std::ptr::null_mut(),
                size: 0,
            }),
        }
    }

    pub fn size(&self) -> usize {
        let state = self.inner.lock().unwrap();
        state.size
    }

    pub fn push_front(&self, frame: Arc<FrameWrpr>) {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        let frame_ptr = Arc::into_raw(frame) as *mut FrameWrpr;

        unsafe {
            (*frame_ptr).next.get().write(state.head);
            (*frame_ptr).prev.get().write(std::ptr::null_mut());
            if !state.head.is_null() {
                (*state.head).prev.get().write(frame_ptr);
            }
            state.head = frame_ptr;
            if state.tail.is_null() {
                state.tail = frame_ptr;
            }
            state.size += 1;
        }
    }

    pub fn push_back(&self, frame: Arc<FrameWrpr>) {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        let frame_ptr = Arc::into_raw(frame) as *mut FrameWrpr;

        unsafe {
            (*frame_ptr).prev.get().write(state.tail);
            (*frame_ptr).next.get().write(std::ptr::null_mut());
            if !state.tail.is_null() {
                (*state.tail).next.get().write(frame_ptr);
            }
            state.tail = frame_ptr;
            if state.head.is_null() {
                state.head = frame_ptr;
            }
            state.size += 1;
        }
    }

    pub fn pop_front(&self) -> Option<Arc<FrameWrpr>> {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;

        if state.head.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = state.head;
            state.head = (*frame_ptr).next.get().read();
            if !state.head.is_null() {
                (*state.head).prev.get().write(std::ptr::null_mut());
            } else {
                state.tail = std::ptr::null_mut();
            }
            state.size -= 1;
            Some(Arc::from_raw(frame_ptr))
        }
    }

    pub fn pop_back(&self) -> Option<Arc<FrameWrpr>> {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;

        if state.tail.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = state.tail;
            state.tail = (*frame_ptr).prev.get().read();
            if !state.tail.is_null() {
                (*state.tail).next.get().write(std::ptr::null_mut());
            } else {
                state.head = std::ptr::null_mut();
            }
            state.size -= 1;
            Some(Arc::from_raw(frame_ptr))
        }
    }
}

impl List {
    pub fn iter(&self) -> ListIter {
        let guard = self.inner.lock().unwrap();
        let state = &*guard;
        ListIter {
            current: state.head,
        }
    }

    pub fn iter_back(&self) -> ListIter {
        let guard = self.inner.lock().unwrap();
        let state = &*guard;
        ListIter {
            current: state.tail,
        }
    }
}

pub struct ListIter {
    current: *mut FrameWrpr,
}

impl Iterator for ListIter {
    type Item = Arc<FrameWrpr>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = self.current;
            self.current = (*frame_ptr).next.get().read();
            Some(Arc::from_raw(frame_ptr))
        }
    }
}

impl DoubleEndedIterator for ListIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = self.current;
            self.current = (*frame_ptr).prev.get().read();
            Some(Arc::from_raw(frame_ptr))
        }
    }
}
