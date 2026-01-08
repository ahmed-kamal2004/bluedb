/// TODO: hashmap isn't safe for concurrent modification (dashmap or rwlock)
/// TODO: flush all the BufPool on drop
/// TODO: dirty pages handling ?
/// TODO: eviction policy and requirements ?
/// TODO: eviction at Arc = 1 ? what about if during disk read or write ?
/// TODO: pinning ?
/// TODO: add error handling in an idomatic way
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::{ Arc, RwLock};
use crate::request::DiskRequest;
use super::disk::DiskManager; // to be used
use super::page::PageFrame;

#[derive(Debug)]
pub struct BufPool {
    map: HashMap<(usize, usize), Arc<RwLock<PageFrame>>>,
    path: HashMap<usize, PathBuf>,
    capacity: usize // not used yet
}


impl BufPool {
    pub fn new() -> Self {
        BufPool { 
            map: HashMap::new(),
            path: HashMap::new(),
            capacity: 0
        }
    }

    pub fn fetch_page(& mut self, req: &DiskRequest) -> Arc<RwLock<PageFrame>> {
        match self.map.entry((req.object_id, req.page_id)) {
            Entry::Occupied(e) => {
                return Arc::clone(e.get());
            },
            Entry::Vacant(e) => {
                let file_path = &self.path[&req.object_id];
                let page_arc = Arc::new(RwLock::new(PageFrame::new()));

                {
                    // read from disk
                    let mut guard = page_arc.write().unwrap();
                    DiskManager::read(&file_path, req.page_id, &mut guard);
                }

                return Arc::clone(e.insert(page_arc));
            }
        }
    }

    pub fn write_page(&self, req: &DiskRequest) {
        let file_path = &self.path[&req.object_id];
        let page_arc = Arc::clone(&self.map[&(req.object_id, req.page_id)]);
        let guard = page_arc.read().unwrap();
        DiskManager::write(file_path, req.page_id, &guard);
    }

    pub fn add_file(& mut self, file_path: &str, file_id: usize) {
        self.path.insert(file_id, PathBuf::from(file_path));
    }

    pub fn get_file(&self, file_id: usize) -> &Path {
        if self.path.contains_key(&file_id) {
             return  &self.path[&file_id]; 
        }
        &Path::new("s")
    }
}


