use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::page::{PageFrame, PageKey};

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
