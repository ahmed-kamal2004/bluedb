use crate::page::PageKey;

pub enum Op {
    READ,
    WRITE,
}

pub struct DiskRequest {
    pub operation: Op,
    pub page_key: PageKey,
}
