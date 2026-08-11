use crate::pool::wrpr::FrameWrpr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

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
            state.head = *(*frame_ptr).next.get();
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
            state.tail = *(*frame_ptr).prev.get();
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

impl<'a> List {
    pub fn iter(&'a self) -> ListIter<'a> {
        let guard = self.inner.lock().unwrap();
        let state = &*guard;
        let head = state.head;
        drop(state);
        ListIter {
            _guard: guard,
            current_next: head,
            current_prev: std::ptr::null_mut(),
        }
    }

    pub fn iter_back(&'a self) -> ListIter<'a> {
        let guard = self.inner.lock().unwrap();
        let state = &*guard;
        let tail = state.tail;
        drop(state);
        ListIter {
            _guard: guard,
            current_next: std::ptr::null_mut(),
            current_prev: tail,
        }
    }
}

pub struct ListIter<'a> {
    _guard: MutexGuard<'a, State>,
    current_next: *mut FrameWrpr,
    current_prev: *mut FrameWrpr,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = Arc<FrameWrpr>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_next.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = self.current_next;
            self.current_next = *(*frame_ptr).next.get();

            let frame_arc = Arc::from_raw(frame_ptr);
            let temp_arc = Arc::clone(&frame_arc);
            let _ = Arc::into_raw(frame_arc); // Prevent dropping the original Arc (doesn't decrement the ref count)
            Some(temp_arc)
        }
    }
}

impl<'a> DoubleEndedIterator for ListIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current_prev.is_null() {
            return None;
        }

        unsafe {
            let frame_ptr = self.current_prev;
            self.current_prev = *(*frame_ptr).prev.get();
            let frame_arc = Arc::from_raw(frame_ptr);
            let temp_arc = Arc::clone(&frame_arc);
            let _ = Arc::into_raw(frame_arc); // Prevent dropping the original Arc
            Some(temp_arc)
        }
    }
}

impl Drop for List {
    fn drop(&mut self) {
        while let Some(frame) = self.pop_front() {
            drop(frame);
        }
    }
}
