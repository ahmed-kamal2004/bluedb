use crate::PAGE_SIZE;
use crate::page::PageFrame;
use libc;
use std::fs::OpenOptions;
use std::fs::read_to_string;
/// TODO: Better error handling mechanisms
/// TODO: keep files open
/// TODO: what about fsync ?
/// TODO: check alignment requirements
use std::os::unix::fs::FileExt;
use std::os::unix::fs::OpenOptionsExt;

#[derive(Debug)]
pub(crate) struct DiskManager;

impl DiskManager {
    pub fn read(file_path: &str, page_num: usize, opage: &mut PageFrame) -> anyhow::Result<()> {
        let mut options = OpenOptions::new();
        options.custom_flags(libc::O_DIRECT).read(true);
        let file = options.open(file_path)?;
        let mut buf = [0; PAGE_SIZE];
        file.read_at(&mut buf, (page_num * PAGE_SIZE) as u64)?;
        *opage.page = buf;
        println!(
            "[Disk Manager] Read page {} from file {}",
            page_num, file_path
        );
        Ok(())
    }

    pub fn write(file_path: &str, page_num: usize, ipage: &PageFrame) -> anyhow::Result<()> {
        let mut options = OpenOptions::new();
        options.custom_flags(libc::O_DIRECT).write(true);
        let file = options.open(file_path)?;
        file.write_at(&*ipage.page, (page_num * PAGE_SIZE) as u64)?;
        println!(
            "[Disk Manager] Written page {} to file {}",
            page_num, file_path
        );
        Ok(())
    }

    pub fn read_file_line_by_line(file_path: &str) -> Vec<String> {
        let mut result = Vec::new();

        for line in read_to_string(file_path).unwrap().lines() {
            result.push(line.to_string())
        }

        result
    }
}
