/// TODO: hashmap isn't safe for concurrent modification (dashmap or rwlock)
/// TODO: flush all the BufPool on drop
/// TODO: dirty pages handling ?
/// TODO: eviction policy and requirements ?
/// TODO: eviction at Arc = 1 ? what about if during disk read or write ?
/// TODO: pinning ?
/// TODO: add error handling in an idomatic way
use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry;
use std::path::{PathBuf};
use std::str::FromStr;
use std::sync::{ Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::{BUFFER_CAPACITY, EVICTION_PERIOD, FILES_METADATA};
use crate::page::PageKey;
use crate::request::DiskRequest;
use super::disk::DiskManager; // to be used
use super::page::PageFrame;
use std::thread;

#[derive(Debug)]
pub struct BufPool {
    page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
    page_id_queue: Arc<RwLock<VecDeque<PageKey>>>,
    path: Arc<RwLock<HashMap<usize, PathBuf>>>,
    metadata_map: Arc<RwLock<HashMap<PageKey, bool>>>,
    capacity: usize, // not used yet
    eviction_thread: Option<JoinHandle<()>>,
}


impl BufPool {
    pub fn new() -> Self {
        BufPool { 
            page_pool: Arc::new(RwLock::new(HashMap::new())),
            page_id_queue: Arc::new(RwLock::new(VecDeque::new())),
            path: Arc::new(RwLock::new(HashMap::new())),
            metadata_map: Arc::new(RwLock::new(HashMap::new())),
            capacity: BUFFER_CAPACITY,
            eviction_thread: None
        }
    }

    // Main buffer pool initialization function
    pub fn initialize() -> Self {
        let mut buf_pool = Self::new();

        buf_pool.load_files();

        buf_pool.eviction_start();

        buf_pool
    }


    pub fn eviction_start(&mut self){ 
        let cloned_page_pool = Arc::clone(&self.page_pool);
        let cloned_path_map = Arc::clone(&self.path);
        let cloned_page_id_queue = Arc::clone(&self.page_id_queue);
        let cloned_metadata_map = Arc::clone(&self.metadata_map);
        let join_evictor_handle: JoinHandle<()> = thread::spawn(
            || {
                Self::eviction_loop(cloned_page_pool, cloned_page_id_queue, cloned_metadata_map, cloned_path_map);
            }
        );
        self.eviction_thread = Some(join_evictor_handle);
    }


    pub fn eviction_loop(page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>, page_id_queue: Arc<RwLock<VecDeque<PageKey>>>, usage_map: Arc<RwLock<HashMap<PageKey, bool>>> , path_map: Arc<RwLock<HashMap<usize, PathBuf>>>) {
        todo!();
        let eviction_map: HashMap<PageKey, PageFrame> = HashMap::new();

        loop {
            thread::sleep(Duration::from_secs(EVICTION_PERIOD as u64));
        }

    }

    // pub fn fetch_page(& mut self, req: &DiskRequest) -> PageKey {
    //     match self.map.entry((req.object_id, req.page_id)) {
    //         Entry::Occupied(e) => {
    //             return Arc::clone(e.get());
    //         },
    //         Entry::Vacant(e) => {
    //             let file_path = &self.path[&req.object_id];
    //             let page_arc = Arc::new(RwLock::new(PageFrame::new()));

    //             {
    //                 // read from disk
    //                 let mut guard = page_arc.write().unwrap();
    //                 DiskManager::read(&file_path, req.page_id, &mut guard);
    //             }

    //             return Arc::clone(e.insert(page_arc));
    //         }
    //     }
    // }

    /// Aquires the lock over the page
    pub fn aquire_page(&mut self, page_key: PageKey) -> Arc<RwLock<PageFrame>> {
        todo!();
    }

    /// Release page with dirty or not
    pub fn release_page(&mut self, page_key: PageKey, made_dirty: bool){
        todo!();
    }

    pub fn load_files(&mut self) {
        let path_vector = DiskManager::read_file_line_by_line(FILES_METADATA);
        let path_hashmap: HashMap<usize, PathBuf> = path_vector
            .iter()
            .filter_map(|line| {
                let mut parts = line.splitn(2, ' '); 
                let key_str = parts.next()?;
                let path_str = parts.next()?;
                let key = key_str.parse::<usize>().ok()?;
                Some((key, PathBuf::from(path_str)))
            })
            .collect();

        *self.path.write().unwrap() = path_hashmap;
    }

    // pub fn write_page(&self, req: &DiskRequest) {
    //     let file_path = &self.path[&req.object_id];
    //     let page_arc = Arc::clone(&self.map[&(req.object_id, req.page_id)]);
    //     let guard = page_arc.read().unwrap();
    //     DiskManager::write(file_path, req.page_id, &guard);
    // }

    // pub fn add_file(& mut self, file_path: &str, file_id: usize) {
    //     self.path.insert(file_id, PathBuf::from(file_path));
    // }

    // pub fn get_file(&self, file_id: usize) -> &Path {
    //     if self.path.contains_key(&file_id) {
    //          return  &self.path[&file_id]; 
    //     }
    //     &Path::new("s")
    // }
}

impl Drop for BufPool {
    fn drop(&mut self) {
        // flush all the dirty pages.
        if let Some(eviction_thread) = self.eviction_thread.take() { // take is used because of the JoinHandle doesn't implement the eviction thread
            eviction_thread.join();
        }
    }
}


