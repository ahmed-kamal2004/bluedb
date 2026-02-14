use crate::PAGE_SIZE;
type FileID = usize;
type PageID = usize;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct PageKey {
    file_id: FileID,
    page_id: PageID,
}

impl PageKey {
    pub fn new(x: FileID, y: PageID) -> Self {
        PageKey {
            file_id: x,
            page_id: y,
        }
    }

    pub fn set_file_id(&mut self, file_id: FileID) {
        self.file_id = file_id;
    }

    pub fn set_page_id(&mut self, page_id: PageID) {
        self.page_id = page_id;
    }

    pub fn get_key(&self) -> (FileID, PageID) {
        (self.file_id, self.page_id)
    }

    pub fn get_file_id(&self) -> FileID {
        self.file_id
    }

    pub fn get_page_id(&self) -> PageID {
        self.page_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageMetaData {
    used_count: usize,
    dirty: bool,
}

impl PageMetaData {
    pub fn new() -> Self {
        PageMetaData {
            used_count: 0,
            dirty: false,
        }
    }

    pub fn use_page(&mut self) {
        self.used_count += 1;
    }

    pub fn release_page(&mut self) {
        self.used_count -= 1;
    }

    pub fn is_used(&self) -> bool {
        self.used_count == 0
    }

    pub fn make_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for PageMetaData {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PageFrame {
    pub page_id: usize, // Limiting the number of pages per table to be usize
    pub metadata: PageMetaData,
    pub page: Box<[u8; PAGE_SIZE]>, // 8 byte -(pointing to)> // 8192 bytes in heap memory
}

impl PageFrame {
    pub fn new() -> Self {
        PageFrame {
            page_id: usize::MIN,
            metadata: PageMetaData::default(),
            page: Box::new([0; PAGE_SIZE]),
        }
    }

    pub fn set_id(&mut self, id: usize) {
        self.page_id = id;
    }
}

impl Default for PageFrame {
    fn default() -> Self {
        Self::new()
    }
}
