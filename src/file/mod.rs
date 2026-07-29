use anyhow::Result;
use libc::O_DIRECT;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::unix::fs::OpenOptionsExt;
use tracing::info;

use crate::constants::FILE_UNIX_PAGE_SIZE;
pub struct FileManager {}

impl FileManager {
    /*File management methods */

    /* Creates */
    pub fn create_file(fl_path: &str) -> Result<()> {
        let mut opts = OpenOptions::new();
        opts.create(true);
        opts.write(true);
        let _fl = opts.open(fl_path)?;
        Ok(())
    }

    pub fn create_dir(dir_path: &str) -> Result<()> {
        std::fs::create_dir_all(dir_path)?;
        Ok(())
    }
    /* Writes */
    pub fn write_to_file(fl_path: &str, offset: u64, mut data: Vec<u8>, size: usize) -> Result<()> {
        if !FileManager::is_file_exists(fl_path) {
            return Err(anyhow::anyhow!(
                "FileManager: File {} does not exist",
                fl_path
            ));
        }

        let data_size = data.len();
        if size < data_size {
            return Err(anyhow::anyhow!(
                "FileManager: File path {}, Size {} is greater than data size {}",
                fl_path,
                size,
                data_size
            ));
        } else {
            let mut data_cloned = data.clone();
            data_cloned.resize(size, ' ' as u8);
            data = data_cloned;
        }

        let mut opts = OpenOptions::new();
        opts.write(true);
        let mut fl = opts.open(fl_path)?;

        fl.seek(std::io::SeekFrom::Start(offset))?;
        fl.write_all(&data)?;
        Ok(())
    }

    pub fn write_to_file_padded(fl_path: &str, offset: u64, data: Vec<u8>) -> Result<()> {
        let data_size = data.len() as u64;
        let remaining_size_to_pad = if data_size % FILE_UNIX_PAGE_SIZE as u64 != 0 {
            FILE_UNIX_PAGE_SIZE as u64 - (data_size % FILE_UNIX_PAGE_SIZE as u64)
        } else {
            0
        };
        FileManager::write_to_file(
            fl_path,
            offset,
            data,
            data_size as usize + remaining_size_to_pad as usize,
        )
    }

    pub fn append_to_file(fl_pth: &str, data: Vec<u8>) -> Result<()> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(());
        }
        FileManager::write_to_file(fl_pth, FileManager::get_file_size(fl_pth)?, data, data_len)
    }

    pub fn write_whole_file(fl_pth: &str, data: Vec<u8>) -> Result<()> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(());
        }
        FileManager::truncate_file(fl_pth, 0)?;
        FileManager::write_to_file(fl_pth, 0, data, data_len)
    }

    /* Reads */
    pub fn read_from_file(fl_pth: &str, offset: u64, size: usize) -> Result<Vec<u8>> {
        if !FileManager::is_file_exists(fl_pth) {
            return Err(anyhow::anyhow!(
                "FileManager: File {} does not exist",
                fl_pth
            ));
        }

        let mut opts = OpenOptions::new();
        opts.read(true);
        let mut fl = opts.open(fl_pth)?;
        let mut buffer = vec![0u8; size];
        fl.seek(std::io::SeekFrom::Start(offset))?;
        fl.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    pub fn read_whole_file(fl_pth: &str) -> Result<Vec<u8>> {
        if !FileManager::is_file_exists(fl_pth) {
            return Err(anyhow::anyhow!(
                "FileManager: File {} does not exist",
                fl_pth
            ));
        }

        let file_size = FileManager::get_file_size(fl_pth)?;
        FileManager::read_from_file(fl_pth, 0, file_size as usize)
    }

    /*Utilities */

    pub fn get_file_size(fl_path: &str) -> Result<u64> {
        if !FileManager::is_file_exists(fl_path) {
            return Err(anyhow::anyhow!(
                "FileManager: File {} does not exist",
                fl_path
            ));
        }

        let mut opts = OpenOptions::new();
        opts.read(true);
        let fl = opts.open(fl_path)?;

        let metadata = fl.metadata()?;
        Ok(metadata.len())
    }

    pub fn truncate_file(fl_path: &str, size: u64) -> Result<()> {
        if !FileManager::is_file_exists(fl_path) {
            return Err(anyhow::anyhow!(
                "FileManager: File {} does not exist",
                fl_path
            ));
        }

        let mut opts = OpenOptions::new();
        opts.write(true);
        let fl = opts.open(fl_path)?;

        fl.set_len(size)?;
        Ok(())
    }

    pub fn delete_file(fl_path: &str) -> Result<()> {
        if !FileManager::is_file_exists(fl_path) {
            return Err(anyhow::anyhow!(
                "FileManager: File {} does not exist",
                fl_path
            ));
        }

        std::fs::remove_file(fl_path)?;
        Ok(())
    }

    pub fn create_directory(dir_path: &str) -> Result<()> {
        std::fs::create_dir_all(dir_path)?;
        Ok(())
    }

    pub fn is_file_exists(fl_path: &str) -> bool {
        std::path::Path::new(fl_path).exists()
    }

    pub fn is_directory_exists(dir_path: &str) -> bool {
        std::path::Path::new(dir_path).is_dir()
    }
}
