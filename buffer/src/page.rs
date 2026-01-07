#[derive(Debug)]

#[repr(C)]
#[derive(Debug)]
pub(crate) PageFrame {
    page_id: usize, // Limiting the number of pages per table to be usize
    page: Box<[u8; PAGE_SIZE]>
}

impl PageFrame {
    pub fn new() -> &Self {
        PageFrame {
            page_id = -1,
            page = Box::new([u8; PAGE_SIZE]) 
        }
    }

    pub fn set_id(&self, id: usize) {
        self.page_id = id;
    }
}