use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use crate::PAGE_SIZE;
use crate::page::PageFrame;

#[derive(Debug)]
pub(crate) struct DiskManager;


impl DiskManager {
    pub fn read(file_path: &Path, page_num: usize, arc_out: &mut Arc<RwLock<PageFrame>>) {
        todo!()
    }
}