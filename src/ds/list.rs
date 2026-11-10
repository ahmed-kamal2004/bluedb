use crate::ds::list_state::ListState;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicUsize;

pub struct List<S> {
    inner: Mutex<S>,
    size: AtomicUsize,
}

unsafe impl<S: ListState + Send> Send for List<S> {}
unsafe impl<S: ListState + Send> Sync for List<S> {}

impl<S: ListState> List<S> {
    pub fn new() -> Self {
        List {
            inner: Mutex::new(S::new()),
            size: AtomicUsize::new(0),
        }
    }

    pub fn size(&self) -> usize {
        self.size.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn push_front(&self, item: S::Item) {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        state.push_front(item);
        self.size.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn push_back(&self, item: S::Item) {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        state.push_back(item);
        self.size.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn pop_front(&self) -> Option<S::Item> {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        match state.pop_front() {
            Some(item) => {
                self.size.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                Some(item)
            }
            None => None,
        }
    }

    pub fn pop_back(&self) -> Option<S::Item> {
        let mut guard = self.inner.lock().unwrap();
        let state = &mut *guard;
        match state.pop_back() {
            Some(item) => {
                self.size.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                Some(item)
            }
            None => None,
        }
    }
}

impl<'a, S: ListState> List<S> {
    pub fn iter(&'a self) -> ListIter<'a, S, S::RawItem> {
        let guard = self.inner.lock().unwrap();
        let state = &*guard;
        let head = state.head();
        let _ = state;
        ListIter {
            _guard: guard,
            current_next: head,
        }
    }
}

pub struct ListIter<'a, SS: ListState, II> {
    _guard: MutexGuard<'a, SS>,
    current_next: II,
}

impl<'a, SS: ListState> Iterator for ListIter<'a, SS, SS::RawItem> {
    type Item = SS::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let s = &*self._guard;
        s.next(&mut self.current_next)
    }
}
