pub enum Op {
    READ,
    WRITE
}


pub struct DiskRequest {
    pub operation: Op,
    pub object_id: usize,
    pub page_id: usize,
}   