use crate::page::PageKey;

#[derive(Debug)]
pub enum Op {
    READ,
    WRITE,
}

#[derive(Debug)]
pub struct DiskRequest {
    pub operation: Op,
    pub page_key: PageKey,
}
