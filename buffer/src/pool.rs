use std::collections::HashMap;
use std::path::Path;
use std::sync::{ Arc, RwLock};
use crate::page;
use crate::request::DiskRequest;
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

    pub fn fetch_page<'a>(& 'a mut self, req: &DiskRequest) -> Arc<RwLock<PageFrame>> {
        if !self.map.contains_key(&(req.object_id, req.page_id)) {
            let file_path = &self.path[&req.object_id];
            let mut page_arc = Arc::new(RwLock::new(PageFrame::new()));
            DiskManager::read(&file_path, req.page_id, &mut page_arc);
            self.map.insert((req.object_id.clone(), req.page_id.clone()), Arc::clone(&page_arc));
            drop(page_arc);
            self.capacity += 1;
        }

        let ptr = &self.map[&(req.object_id, req.page_id)];
        let arc_clone = Arc::clone(ptr);
        arc_clone
    }
}


