use crate::pool::wrpr::FrameWrpr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;

pub struct FlushList {
    inner: Mutex<State>,
    size: AtomicUsize,
}

pub struct State {
    head: *mut FrameWrpr,
}

unsafe impl Send for FlushList {}
unsafe impl Sync for FlushList {}

impl FlushList {
    pub fn new() -> Self {
        FlushList {
            inner: Mutex::new(State {
                head: std::ptr::null_mut(),
            }),
            size: AtomicUsize::new(0),
        }
    }

    pub fn size(&self) -> usize {
        self.size.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn push_by_oldest_modification_data(&self, frame: Arc<FrameWrpr>) {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        let frame_ptr = Arc::into_raw(frame) as *mut FrameWrpr;

        unsafe {
            let mut iter = state.head;
            let mut prev_iter = std::ptr::null_mut();
            while !iter.is_null() {
                if (*iter).oldest_modification_data > (*frame_ptr).oldest_modification_data {
                    break;
                }
                prev_iter = iter;
                iter = *(*iter).next_flush.get();
            }

            if (!iter.is_null()) {
                (*frame_ptr).next_flush.get().write(iter);
            } else {
                (*frame_ptr).next_flush.get().write(std::ptr::null_mut());
            }
            if (!prev_iter.is_null()) {
                (*prev_iter).next_flush.get().write(frame_ptr);
            } else {
                state.head = frame_ptr;
            }

            self.size.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
            state.head = *(*frame_ptr).next_flush.get();
            self.size.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            Some(Arc::from_raw(frame_ptr))
        }
    }
}


mod tests {
    use super::*;

    #[test]
    fn test_flush_list() {
        let flush_list = FlushList::new();

        let mut frame1 = FrameWrpr::new_empty();
        frame1.oldest_modification_data = 10;
        flush_list.push_by_oldest_modification_data(Arc::new(frame1));

        let mut frame2 = FrameWrpr::new_empty();
        frame2.oldest_modification_data = 5;
        flush_list.push_by_oldest_modification_data(Arc::new(frame2));

        let mut frame3 = FrameWrpr::new_empty();
        frame3.oldest_modification_data = 15;
        flush_list.push_by_oldest_modification_data(Arc::new(frame3));

        assert_eq!(flush_list.size(), 3);

        let popped_frame1 = flush_list.pop_front().unwrap();
        assert_eq!(popped_frame1.oldest_modification_data, 5);

        let popped_frame2 = flush_list.pop_front().unwrap();
        assert_eq!(popped_frame2.oldest_modification_data, 10);

        let popped_frame3 = flush_list.pop_front().unwrap();
        assert_eq!(popped_frame3.oldest_modification_data, 15);

        assert_eq!(flush_list.size(), 0);
    }
}