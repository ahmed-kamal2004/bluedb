use std::os::unix::fs::FileExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use crate::PAGE_SIZE;
use crate::page::PageFrame;
use std::os::unix::fs::OpenOptionsExt;
use std::fs::OpenOptions;
use libc;

#[derive(Debug)]
pub(crate) struct DiskManager;


impl DiskManager {
    pub fn read(file_path: &Path, page_num: usize, arc_out: &mut Arc<RwLock<PageFrame>>) {
        let mut options = OpenOptions::new();
        options.custom_flags(libc::O_DIRECT);
        let file = options.open(file_path).unwrap();
        let mut buf = [0; PAGE_SIZE];
        file.read_at(&mut buf, (page_num * PAGE_SIZE) as u64);
        let mut lock = arc_out.write().unwrap();
        (*lock).page = Box::new(buf);
    }
}