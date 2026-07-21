use std::sync::RwLock;
use std::sync::Arc;
use super::frame::Frame;

pub struct FrameWrpr {
    pub frame: RwLock<Frame>,
    pub next: Option<Arc<FrameWrpr>>,
    pub is_dirty: bool,
    pub is_pinned: bool,
    pub is_in_use: bool,
}


pub struct FrameList {
    pub head: Option<Arc<FrameWrpr>>,
    pub tail: Option<Arc<FrameWrpr>>,
    pub size: usize,
}
struct BufPl {
    pub old_list: FrameList,
    pub new_list: FrameList,
    pub max_mem_usage: usize,
    pub current_mem_usage: usize,
}