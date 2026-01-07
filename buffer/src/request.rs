pub(crate) enum Op {
    READ,
    WRITE
}


pub(crate) struct DiskRequest {
    operation: Op,
    page_id: i32,
    // page_ptr: 
}   