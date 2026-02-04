use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::page::{PageFrame, PageKey};

#[derive(Debug)]
pub enum LockGuard<'a> {
    READ(
        RwLockReadGuard<'a, PageFrame>,
        Arc<RwLock<PageFrame>>,
        PageKey,
    ),
    WRITE(
        RwLockWriteGuard<'a, PageFrame>,
        Arc<RwLock<PageFrame>>,
        PageKey,
    ),
}
