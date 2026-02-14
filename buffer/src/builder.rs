use crate::pool::BufPool;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

fn default_file_info() -> String {
    "table-info".to_string()
}

fn default_directory() -> String {
    "/var/lib/bluedb/data".to_string()
}

fn default_eviction_period() -> u8 {
    1
}

fn default_buffer_capacity() -> u32 {
    // assuming default page size is 8KB
    // then for the buffer pool with capacity 10000 capacity size
    // ~ 80 Mb of data
    10240
}

fn default_eviction_threshold() -> u8 {
    0
}

pub struct BufPoolBuilder {
    file_info: String,      // file that contains all database info
    directory_data: String, // directory that contains all the data (similar to pgdata)
    eviction_period: u8,    // time where the thread becomes sleep
    buffer_capacity: u32,   // num of pages only within the pool
    eviction_threshold: u8, // number of times the page can be found unused during eviction.
}

impl BufPoolBuilder {
    pub fn new() -> Self {
        BufPoolBuilder {
            file_info: default_file_info(),
            directory_data: default_directory(),
            eviction_period: default_eviction_period(),
            buffer_capacity: default_buffer_capacity(),
            eviction_threshold: default_eviction_threshold(),
        }
    }

    pub fn build(&self) -> BufPool {
        let mut pool = BufPool {
            page_pool: Arc::new(RwLock::new(HashMap::new())),
            page_id_queue: Arc::new(RwLock::new(VecDeque::new())),
            path: Arc::new(RwLock::new(HashMap::new())),
            usage_map: Arc::new(RwLock::new(HashMap::new())),
            eviction_thread: None,
            file_info: self.file_info.clone(),
            directory_data: self.directory_data.clone(),
            eviction_period: self.eviction_period,
            buffer_capacity: self.buffer_capacity,
            eviction_threshold: self.eviction_threshold,
        };

        pool.initialize();

        pool
    }

    pub fn set_file_info(mut self, file_info: String) -> Self {
        self.file_info = file_info;
        self
    }

    pub fn set_directory_data(mut self, dir: String) -> Self {
        self.directory_data = dir;
        self
    }

    pub fn set_eviction_period(mut self, evc_pir: u8) -> Self {
        self.eviction_period = evc_pir;
        self
    }

    pub fn set_buffer_capacity(mut self, capacity: u32) -> Self {
        self.buffer_capacity = capacity;
        self
    }

    pub fn set_eviction_threshold(mut self, thre: u8) -> Self {
        self.eviction_threshold = thre;
        self
    }
}

impl Default for BufPoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}
