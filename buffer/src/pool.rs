use std::collections::HashMap;
use std::path::Path;
use std::sync::{ Arc, RwLock};
use crate::request::DiskRequest;
use crate::utils::PageGuard;
use crate::request::Op;
use super::disk::DiskManager; // to be used
use super::page::PageFrame;

#[derive(Debug)]
pub struct BufPool {
    map: HashMap<(usize, usize), Arc<RwLock<PageFrame>>>,
    path: HashMap<usize, Box<Path>>,
    capacity: usize
}


impl BufPool {
    pub fn new() -> Self {
        BufPool { 
            map: HashMap::new(),
            path: HashMap::new(),
            capacity: 0
        }
    }

    pub fn request<'a>(& 'a mut self, req: &DiskRequest) -> PageGuard<'a> {
        unimplemented!();
        if !self.map.contains_key(&(req.object_id, req.page_id)) {
            let file_path = &self.path[&req.object_id];
            // let page_arc = Arc::new(RwLock::new(PageFrame::new()));
            // DiskManager::read(&file_path, req.page_id);
            self.capacity += 1;
        }

        match req.operation {
            Op::WRITE => {
                let ptr = self.map[&(req.object_id, req.page_id)].read().unwrap();
                PageGuard::Read(ptr)
            },
            Op::READ => {
                let ptr = self.map[&(req.object_id, req.page_id)].write().unwrap();
                PageGuard::Write(ptr)
            }
        }
    }
}



// use std::sync::{Arc, RwLock};

// let shared = Arc::new(RwLock::new(42));

// // Reader (waits if a writer holds the lock)
// let r = shared.read().unwrap();

// // Writer (waits until all readers release)
// drop(r);
// let mut w = shared.write().unwrap();
// *w = 100;



