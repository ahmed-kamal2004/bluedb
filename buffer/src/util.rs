use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use disk::page::PageFrame;
use disk::page::PageKey;

#[derive(Debug)]
pub enum LockGuard<'a> {
    Read(
        RwLockReadGuard<'a, PageFrame>,
        Arc<RwLock<PageFrame>>,
        PageKey,
    ),
    Write(
        RwLockWriteGuard<'a, PageFrame>,
        Arc<RwLock<PageFrame>>,
        PageKey,
    ),
}
