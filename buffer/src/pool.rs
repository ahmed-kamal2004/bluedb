use std::collections::HashMap;

pub struct BufPool {
    map: HashMap<(usize, usize), PageFrame>,
    capacity: usize,
    disk_manager: DiskManager
}


impl BufPool {
    pub fn read();
}


