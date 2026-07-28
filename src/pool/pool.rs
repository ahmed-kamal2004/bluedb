use super::frame::Frame;
use std::sync::Arc;
use std::sync::RwLock;

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
    pub old_list: Arc<RwLock<FrameList>>,
    pub new_list: Arc<RwLock<FrameList>>,
    pub max_mem_usage: usize,
    pub current_mem_usage: usize,
}
