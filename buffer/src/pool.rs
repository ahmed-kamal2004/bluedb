use super::request::Op;
use crate::builder::BufPoolBuilder;
use crate::request::BufferRequest;
use crate::util::EvictionTimeOperation;
use crate::util::LockGuard;
use disk::disk::DiskManager;
use disk::page::PageFrame;
use disk::page::PageKey;
/// TODO: flush all the BufPool on drop
/// TODO: pinning ?
/// TODO: add error handling in an idomatic way
/// TODO: implement different eviction policies
/// TODO: add file metadata management
/// TODO: improve logging
/// TODO: create a wrapper that understands databases over it.
/// 
/// TODO: instead of acquiring locks, what about an approach to just have counters for write and read transactions using it, as the current
/// approach violates some rust compile time principles, can be very dangerous and bottle-neck.
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug)]
pub struct BufPool {
    pub(crate) page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
    pub(crate) page_id_queue: Arc<RwLock<VecDeque<PageKey>>>, // for eviction policy
    pub(crate) path: Arc<RwLock<HashMap<usize, PathBuf>>>,    // for file pathes
    pub(crate) usage_map: Arc<RwLock<HashMap<PageKey, AtomicUsize>>>, // track page usage ()
    pub(crate) eviction_thread: Option<JoinHandle<()>>,
    pub(crate) file_info: String, // file that contains all database info
    pub(crate) directory_data: String, // directory that contains all the data (similar to pgdata)
    pub(crate) eviction_period: u8, // time where the thread becomes sleep
    pub(crate) buffer_capacity: u32, // num of pages only within the pool
    pub(crate) eviction_threshold: u8, // number of times the page can be found unused during eviction.
}

impl BufPool {
    pub fn builder() -> BufPoolBuilder {
        BufPoolBuilder::new()
    }

    // Main buffer pool initialization function
    pub fn initialize(&mut self) {
        self.load_files();

        self.eviction_start();
    }

    pub fn eviction_start(&mut self) {
        let cloned_page_pool = Arc::clone(&self.page_pool);
        let cloned_path_map = Arc::clone(&self.path);
        let cloned_page_id_queue = Arc::clone(&self.page_id_queue);
        let cloned_metadata_map = Arc::clone(&self.usage_map);
        let eviction_period = self.eviction_period;
        let eviction_threshold = self.eviction_threshold;
        let max_capacity = self.buffer_capacity as usize;
        let join_evictor_handle: JoinHandle<()> = thread::spawn(move || {
            println!("[Eviction] Eviction Thread Started");
            Self::eviction_loop(
                cloned_page_pool,
                cloned_page_id_queue,
                cloned_metadata_map,
                cloned_path_map,
                eviction_period,
                eviction_threshold,
                max_capacity
            );
        });
        self.eviction_thread = Some(join_evictor_handle);
    }

    pub fn eviction_loop(
        page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
        page_id_queue: Arc<RwLock<VecDeque<PageKey>>>,
        usage_map: Arc<RwLock<HashMap<PageKey, AtomicUsize>>>,
        path_map: Arc<RwLock<HashMap<usize, PathBuf>>>,
        mut eviction_period: u8,
        eviction_threshold: u8,
        max_capacity: usize
    ) {
        let mut eviction_map: HashMap<PageKey, u8> = HashMap::new();

        loop {
            thread::sleep(Duration::from_secs(eviction_period as u64));
            let time_op = Self::evict(
                Arc::clone(&page_pool),
                Arc::clone(&page_id_queue),
                Arc::clone(&usage_map),
                Arc::clone(&path_map),
                &mut eviction_map,
                eviction_threshold,
                max_capacity
            ).expect("Eviction failed");
            match time_op {
                EvictionTimeOperation::DOUBLE => {
                    if eviction_period >= 64 { // one minute is the max eviction period
                        continue;
                    }
                    eviction_period = eviction_period * 2;
                    println!("[Eviction] Doubling eviction period to {}", eviction_period);
                }
                EvictionTimeOperation::HALVE => {
                    if eviction_period == 1 {
                        continue;
                    }
                    eviction_period = if eviction_period > 1 { eviction_period / 2 } else { 1 };
                    println!("[Eviction] Halving eviction period to {}", eviction_period);
                }
                _ => {},
            }
        }
    }

    fn evict(
        page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
        page_id_queue: Arc<RwLock<VecDeque<PageKey>>>,
        usage_map: Arc<RwLock<HashMap<PageKey, AtomicUsize>>>,
        path_map: Arc<RwLock<HashMap<usize, PathBuf>>>,
        eviction_map: &mut HashMap<PageKey, u8>,
        eviction_threshold: u8,
        max_capacity: usize,
    ) -> anyhow::Result<EvictionTimeOperation>{
        let mut usage_map_mutable_lock = usage_map.write().unwrap(); // prevents writes to it.
        let mut queue_mutable_lock = page_id_queue.write().unwrap(); // prevents read or writes to it.
        let mut size_of_queue = queue_mutable_lock.len();
        for _i in 0..size_of_queue {
            let front_id_in_queue = queue_mutable_lock.pop_back().unwrap();

            // check eviction logic
            // if it is not used now -> then warn it, it num of warns is greater than a specific value then evict it
            eviction_map.entry(front_id_in_queue).or_insert(0);
            if usage_map_mutable_lock[&front_id_in_queue].load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                let mut curr_eviction_counter = eviction_map[&front_id_in_queue];
                curr_eviction_counter += 1;
                if curr_eviction_counter >= eviction_threshold {
                    size_of_queue -= 1; // decrease the size of the queue when evicting the page from it.
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
                            match DiskManager::write_page(
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
        let empty_percent = (max_capacity - size_of_queue) as f64 / (max_capacity as f64);
        if empty_percent >= 2.0 / 3.0 { // two thirds of the pool is empty, we can increase the eviction period to save resources.
            Ok(EvictionTimeOperation::DOUBLE)
        } else if empty_percent <= 1.0 / 3.0 { // two thirds of the pool is full, we need to decrease the eviction period to increase the eviction frequency and free some space.
            Ok(EvictionTimeOperation::HALVE)
        } else { // no decision needed
            Ok(EvictionTimeOperation::NONE)
        }
    }

    // Acquires read over the pool at the start.
    // if th page exists -> acquire read over the usage then acquire the page lock and return it.
    // if the page doesn't exist -> drop the pool read lock and acquire write lock over the queue, usage and pool
    pub fn acquire_page(&self, req: &BufferRequest) -> anyhow::Result<LockGuard<'_>> {
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
                            let page_arc = Arc::new(RwLock::new(PageFrame::default()));
                            {
                                // read from disk
                                let mut guard = page_arc.write().unwrap();
                                DiskManager::read_page(
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
                let guard = unsafe {
                    std::mem::transmute::<
                        std::sync::RwLockReadGuard<'_, PageFrame>,
                        std::sync::RwLockReadGuard<'_, PageFrame>,
                    >(page.read().unwrap())
                };
                println!(
                    "[BufPool] Acquired READ lock for PageKey: {:?}",
                    req.page_key
                );
                Ok(LockGuard::Read(guard, arc_clone, req.page_key))
            }
            Op::WRITE => {
                let arc_clone = Arc::clone(&page);
                let guard = unsafe {
                    std::mem::transmute::<
                        std::sync::RwLockWriteGuard<'_, PageFrame>,
                        std::sync::RwLockWriteGuard<'_, PageFrame>,
                    >(page.write().unwrap())
                };
                println!(
                    "[BufPool] Acquired WRITE lock for PageKey: {:?}",
                    req.page_key
                );
                Ok(LockGuard::Write(guard, arc_clone, req.page_key))
            }
        }
    }

    fn acquire_existed_page(&self, req: &BufferRequest) -> Option<Arc<RwLock<PageFrame>>> {
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
        let path_vector = DiskManager::read_file_line_by_line(&self.file_info);
        let path_hashmap: HashMap<usize, PathBuf> = path_vector
            .iter()
            .filter_map(|line| {
                let (key_str, path_str) = line.split_once(' ')?;
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