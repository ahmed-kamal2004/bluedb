/// TODO: Better error handling mechanisms
/// TODO: keep files open
/// TODO: what about fsync ?
/// TODO: check alignment requirements
use std::os::unix::fs::FileExt;
use std::path::Path;
use crate::PAGE_SIZE;
use crate::page::PageFrame;
use std::os::unix::fs::OpenOptionsExt;
use std::fs::OpenOptions;
use libc;

#[derive(Debug)]
pub(crate) struct DiskManager;


impl DiskManager {
    pub fn read(file_path: &Path, page_num: usize, opage: &mut PageFrame) {
        let mut options = OpenOptions::new();
        options.custom_flags(libc::O_DIRECT).read(true);
        let file = options.open(file_path).unwrap();
        let mut buf = [0; PAGE_SIZE];
        file.read_at(&mut buf, (page_num * PAGE_SIZE) as u64);
        opage.page = Box::new(buf);
    }

    pub fn write(file_path: &Path, page_num: usize, ipage: &PageFrame) {
        let mut options = OpenOptions::new();
        options.custom_flags(libc::O_DIRECT).write(true);
        let file = options.open(file_path).unwrap();
        file.write_at(&*ipage.page, (page_num * PAGE_SIZE) as u64);
    }
}