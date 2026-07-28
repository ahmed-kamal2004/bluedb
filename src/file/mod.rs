use anyhow::Result;
use libc::O_DIRECT;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::unix::fs::OpenOptionsExt;

use crate::constants::FILE_UNIX_PAGE_SIZE;
pub struct FileManager {}

impl FileManager {
    /*File management methods */

    /// Append data to the end of the file specified by `fl_pth`.
    /// If the file does not exist, it will be created.
    pub fn append_to_file(fl_pth: &str, data: Vec<u8>) -> Result<()> {
        FileManager::write_to_file(fl_pth, FileManager::get_file_size(fl_pth)?, data)
    }

    /// Read data from the file specified by `fl_pth` starting at `offset` and reading `size` bytes.
    pub fn read_from_file(fl_pth: &str, offset: u64, size: usize) -> Result<Vec<u8>> {
        let mut opts = OpenOptions::new();
        opts.read(true);
        opts.custom_flags(O_DIRECT);
        let mut fl = opts.open(fl_pth)?;
        let mut buffer = vec![0u8; size];
        fl.seek(std::io::SeekFrom::Start(offset))?;
        fl.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    pub fn read_whole_file(fl_pth: &str) -> Result<Vec<u8>> {
        let mut opts = OpenOptions::new();
        opts.create(true);
        opts.read(true);
        opts.write(true);
        let mut fl = opts.open(fl_pth)?;

        if FileManager::get_file_size(fl_pth)? == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = Vec::new();
        fl.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    /// Write data to the file specified by `fl_path` starting at `offset`.
    pub fn write_to_file(fl_path: &str, offset: u64, data: Vec<u8>) -> Result<()> {
        let mut opts = OpenOptions::new();
        opts.write(true);
        opts.custom_flags(O_DIRECT);
        let mut fl = opts.open(fl_path)?;

        fl.seek(std::io::SeekFrom::Start(offset))?;
        fl.write_all(&data)?;
        Ok(())
    }

    pub fn write_to_file_padded(
        fl_path: &str,
        offset: u64,
        mut data: Vec<u8>,
        size: usize,
    ) -> Result<()> {
        let data_size = data.len() as u64;
        let remaining_size_to_pad = if data_size % FILE_UNIX_PAGE_SIZE as u64 != 0 {
            FILE_UNIX_PAGE_SIZE as u64 - (data_size % FILE_UNIX_PAGE_SIZE as u64)
        } else {
            0
        };

        if remaining_size_to_pad > 0 {
            let mut padded_data = data.clone();
            padded_data.resize((data_size + remaining_size_to_pad) as usize, 0);
            data = padded_data;
        }

        let mut opts = OpenOptions::new();
        opts.write(true);
        opts.custom_flags(O_DIRECT);
        let mut fl = opts.open(fl_path)?;

        fl.seek(std::io::SeekFrom::Start(offset))?;
        fl.write_all(&data)?;
        Ok(())
    }

    pub fn extend_file(fl_path: &str, size: u64) -> Result<()> {
        let mut opts = OpenOptions::new();
        opts.write(true);
        // opts.custom_flags(O_DIRECT);
        let mut fl = opts.open(fl_path)?;

        fl.set_len(size)?; // direct IO is removed as this functions maps to 
        Ok(())
    }

    pub fn get_file_size(fl_path: &str) -> Result<u64> {
        let mut opts = OpenOptions::new();
        opts.read(true);
        let fl = opts.open(fl_path)?;

        let metadata = fl.metadata()?;
        Ok(metadata.len())
    }

    pub fn delete_file(fl_path: &str) -> Result<()> {
        std::fs::remove_file(fl_path)?;
        Ok(())
    }
}
