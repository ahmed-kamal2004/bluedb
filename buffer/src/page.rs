use std::usize;

use super::PAGE_SIZE;

#[repr(C)]
#[derive(Debug)]
pub struct PageFrame {
    pub page_id: usize, // Limiting the number of pages per table to be usize
    pub page: Box<[u8; PAGE_SIZE]>
}

impl PageFrame {
    pub fn new() -> Self {
        PageFrame {
            page_id: usize::MIN,
            page: Box::new([0; PAGE_SIZE]) 
        }
    }

    pub fn set_id(&mut self, id: usize) {
        self.page_id = id;
    }
}