use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use super::page::PageFrame;
pub enum PageGuard<'a> {
    Read(RwLockReadGuard<'a, PageFrame>),
    Write(RwLockWriteGuard<'a, PageFrame>),
}