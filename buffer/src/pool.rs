use super::disk::DiskManager;
use super::page::PageFrame;
use super::request::Op;
use crate::page::PageKey;
use crate::request::DiskRequest;
use crate::util::LockGuard;
use crate::{EVICTION_PERIOD, EVICTION_THREASHOLD, FILES_METADATA};
/// TODO: flush all the BufPool on drop
/// TODO: pinning ?
/// TODO: add error handling in an idomatic way
/// TODO: implement different eviction policies
/// TODO: improve concurrency (this is prune to deadlocks) (most important part) [DONE]
/// TODO: add file metadata management
/// TODO: improve logging
/// TODO: create a wrapper that understands databases over it.
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug)]
pub struct BufPool {
    page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
    page_id_queue: Arc<RwLock<VecDeque<PageKey>>>, // for eviction policy
    path: Arc<RwLock<HashMap<usize, PathBuf>>>,    // for file pathes
    usage_map: Arc<RwLock<HashMap<PageKey, AtomicUsize>>>, // track page usage ()
    eviction_thread: Option<JoinHandle<()>>,
}

impl BufPool {
    pub fn new() -> Self {
        BufPool {
            page_pool: Arc::new(RwLock::new(HashMap::new())),
            page_id_queue: Arc::new(RwLock::new(VecDeque::new())),
            path: Arc::new(RwLock::new(HashMap::new())),
            usage_map: Arc::new(RwLock::new(HashMap::new())),
            eviction_thread: None,
        }
    }

    // Main buffer pool initialization function
    pub fn initialize() -> Self {
        let mut buf_pool = Self::new();

        buf_pool.load_files();

        buf_pool.eviction_start();

        buf_pool
    }

    pub fn eviction_start(&mut self) {
        let cloned_page_pool = Arc::clone(&self.page_pool);
        let cloned_path_map = Arc::clone(&self.path);
        let cloned_page_id_queue = Arc::clone(&self.page_id_queue);
        let cloned_metadata_map = Arc::clone(&self.usage_map);
        let join_evictor_handle: JoinHandle<()> = thread::spawn(|| {
            println!("[Eviction] Eviction Thread Started");
            Self::eviction_loop(
                cloned_page_pool,
                cloned_page_id_queue,
                cloned_metadata_map,
                cloned_path_map,
            );
        });
        self.eviction_thread = Some(join_evictor_handle);
    }

    pub fn eviction_loop(
        page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
        page_id_queue: Arc<RwLock<VecDeque<PageKey>>>,
        usage_map: Arc<RwLock<HashMap<PageKey, AtomicUsize>>>,
        path_map: Arc<RwLock<HashMap<usize, PathBuf>>>,
    ) {
        let mut eviction_map: HashMap<PageKey, u32> = HashMap::new();

        loop {
            thread::sleep(Duration::from_secs(EVICTION_PERIOD as u64));
            Self::evict(
                Arc::clone(&page_pool),
                Arc::clone(&page_id_queue),
                Arc::clone(&usage_map),
                Arc::clone(&path_map),
                &mut eviction_map,
            );
        }
    }

    fn evict(
        page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
        page_id_queue: Arc<RwLock<VecDeque<PageKey>>>,
        usage_map: Arc<RwLock<HashMap<PageKey, AtomicUsize>>>,
        path_map: Arc<RwLock<HashMap<usize, PathBuf>>>,
        eviction_map: &mut HashMap<PageKey, u32>,
    ) {
        let mut usage_map_mutable_lock = usage_map.write().unwrap(); // prevents writes to it.
        let mut queue_mutable_lock = page_id_queue.write().unwrap(); // prevents read or writes to it.
        let size_of_queue = queue_mutable_lock.len();
        for _i in 0..size_of_queue {
            let front_id_in_queue = queue_mutable_lock.pop_back().unwrap();

            // check eviction logic
            // if it is not used now -> then warn it, it num of warns is greater than a specific value then evict it
            if !eviction_map.contains_key(&front_id_in_queue) {
                eviction_map.insert(front_id_in_queue, 0);
            }
            if usage_map_mutable_lock[&front_id_in_queue].load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                let mut curr_eviction_counter = eviction_map[&front_id_in_queue];
                curr_eviction_counter += 1;
                if curr_eviction_counter >= EVICTION_THREASHOLD {
                    eviction_map.remove(&front_id_in_queue); // remove from the single threaded eviction map
                    usage_map_mutable_lock.remove(&front_id_in_queue);
                    {
                        // aquire write lock on the page pool
                        let mut page_mutable_lock = page_pool.write().unwrap();
                        let page = page_mutable_lock.remove(&front_id_in_queue).unwrap();
                        let guard = page.read().unwrap();
                        if guard.metadata.is_dirty() {
                            let path_map_lock = path_map.read().unwrap();
                            let file_path = &path_map_lock[&front_id_in_queue.get_file_id()];
                            match DiskManager::write(
                                file_path.to_str().unwrap(),
                                front_id_in_queue.get_page_id(),
                                &guard,
                            ) {
                                Ok(_) => {
                                    println!(
                                        "[Eviction] Flushed dirty page {:?} to disk successfully",
                                        front_id_in_queue
                                    );
                                }
                                Err(e) => {
                                    println!(
                                        "[Eviction] Error flushing dirty page {:?} to disk: {:?}",
                                        front_id_in_queue, e
                                    );
                                }
                            }
                        }
                    }
                } else {
                    let eviction_val = eviction_map.get_mut(&front_id_in_queue).unwrap();
                    *eviction_val = curr_eviction_counter;
                }
            } else {
                queue_mutable_lock.push_back(front_id_in_queue);
                let eviction_val = eviction_map.get_mut(&front_id_in_queue).unwrap();
                *eviction_val = 0;
            }
        }
    }

    // Acquires read over the pool at the start.
    // if th page exists -> acquire read over the usage then acquire the page lock and return it.
    // if the page doesn't exist -> drop the pool read lock and acquire write lock over the queue, usage and pool
    pub fn acquire_page(&self, req: &DiskRequest) -> anyhow::Result<LockGuard> {
        let page: Arc<RwLock<PageFrame>> = {
            match self.acquire_existed_page(req) {
                Some(page) => page,
                None => {
                    // page doesn't exist in the pool, we need to fetch it from the disk

                    // TODO: verify file metadata, check that this page is already exists in the file, if not create it and add it to the file metadata

                    //acquire locks over the queue and usage map, this ensures no eviction is going now.
                    let mut queue_lock = self.page_id_queue.write().unwrap();
                    let mut usage_map_mutable_lock = self.usage_map.write().unwrap();

                    // do a final check before retrieving the page from the disk to make sure it wasn't added by another thread while we were waiting for the locks
                    match self.acquire_existed_page(req) {
                        Some(page) => page,
                        None => {
                            let mut pool_write = self.page_pool.write().unwrap();
                            let file_path = {
                                let path_lock = self.path.read().unwrap();
                                path_lock[&req.page_key.get_file_id()].clone()
                            };
                            let page_arc = Arc::new(RwLock::new(PageFrame::new()));
                            {
                                // read from disk
                                let mut guard = page_arc.write().unwrap();
                                DiskManager::read(
                                    file_path.to_str().unwrap(),
                                    req.page_key.get_page_id(),
                                    &mut guard,
                                )
                                .expect("Failed to read page from disk");
                            }
                            pool_write.insert(req.page_key, Arc::clone(&page_arc));
                            queue_lock.push_front(req.page_key);
                            usage_map_mutable_lock.insert(req.page_key, AtomicUsize::new(1));
                            page_arc
                        }
                    }
                }
            }
        };

        match req.operation {
            Op::READ => {
                let arc_clone = Arc::clone(&page);
                let guard = unsafe { std::mem::transmute(page.read().unwrap()) };
                println!(
                    "[BufPool] Acquired READ lock for PageKey: {:?}",
                    req.page_key
                );
                Ok(LockGuard::Read(guard, arc_clone, req.page_key))
            }
            Op::WRITE => {
                let arc_clone = Arc::clone(&page);
                let guard = unsafe { std::mem::transmute(page.write().unwrap()) };
                println!(
                    "[BufPool] Acquired WRITE lock for PageKey: {:?}",
                    req.page_key
                );
                Ok(LockGuard::Write(guard, arc_clone, req.page_key))
            }
        }
    }

    fn acquire_existed_page(&self, req: &DiskRequest) -> Option<Arc<RwLock<PageFrame>>> {
        let pool_read = self.page_pool.read().unwrap();
        if pool_read.contains_key(&req.page_key) {
            let page_frame = pool_read.get(&req.page_key).unwrap();
            let usage_map_read = self.usage_map.read().unwrap();
            let page_usage = usage_map_read.get(&req.page_key).unwrap();
            page_usage.store(
                page_usage.load(std::sync::atomic::Ordering::SeqCst) + 1,
                std::sync::atomic::Ordering::SeqCst,
            );
            Some(Arc::clone(page_frame))
        } else {
            None
        }
    }

    // Acquires Read lock over the usage map only to decrement it.
    // then drops the page lock
    pub fn release_page(&self, guard: LockGuard) {
        let usage_map_read_lock = self.usage_map.read().unwrap(); // prevents writes to it.
        match guard {
            LockGuard::Read(_, _, key) => {
                let page_usage = usage_map_read_lock.get(&key).unwrap();
                page_usage.store(
                    page_usage.load(std::sync::atomic::Ordering::SeqCst) - 1,
                    std::sync::atomic::Ordering::SeqCst,
                );
                println!("[BufPool] Released READ lock for PageKey: {:?}", key);
            }
            LockGuard::Write(mut write_lock, _, key) => {
                write_lock.metadata.make_dirty();
                let page_usage = usage_map_read_lock.get(&key).unwrap();
                page_usage.store(
                    page_usage.load(std::sync::atomic::Ordering::SeqCst) - 1,
                    std::sync::atomic::Ordering::SeqCst,
                );
                println!("[BufPool] Released WRITE lock for PageKey: {:?}", key);
            }
        }
    }

    pub fn load_files(&self) {
        let path_vector = DiskManager::read_file_line_by_line(FILES_METADATA);
        let path_hashmap: HashMap<usize, PathBuf> = path_vector
            .iter()
            .filter_map(|line| {
                let mut parts = line.splitn(2, ' ');
                let key_str = parts.next()?;
                let path_str = parts.next()?;
                let key = key_str.parse::<usize>().ok()?;
                Some((key, PathBuf::from(path_str)))
            })
            .collect();

        *self.path.write().unwrap() = path_hashmap;
    }
}

impl Drop for BufPool {
    fn drop(&mut self) {
        // flush all the dirty pages.
        if let Some(eviction_thread) = self.eviction_thread.take() {
            // take is used because of the JoinHandle doesn't implement the eviction thread
            match eviction_thread.join() {
                Ok(_) => {
                    println!("[BufPool] Eviction thread joined successfully on drop.");
                }
                Err(e) => {
                    println!("[BufPool] Error joining eviction thread on drop: {:?}", e);
                }
            }
        }
    }
}
