use super::disk::DiskManager; // to be used
use super::page::PageFrame;
use super::request::Op;
use crate::page::PageKey;
use crate::request::DiskRequest;
use crate::util::LockGuard;
use crate::{BUFFER_CAPACITY, EVICTION_PERIOD, EVICTION_THREASHOLD, FILES_METADATA, page};
use std::collections::hash_map::Entry;
/// TODO: hashmap isn't safe for concurrent modification (dashmap or rwlock)
/// TODO: flush all the BufPool on drop
/// TODO: dirty pages handling ?
/// TODO: eviction policy and requirements ?
/// TODO: eviction at Arc = 1 ? what about if during disk read or write ?
/// TODO: pinning ?
/// TODO: add error handling in an idomatic way
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug)]
pub struct BufPool {
    page_pool: Arc<RwLock<HashMap<PageKey, Arc<RwLock<PageFrame>>>>>,
    page_id_queue: Arc<RwLock<VecDeque<PageKey>>>, // for eviction policy
    path: Arc<RwLock<HashMap<usize, PathBuf>>>,    // for file pathes
    usage_map: Arc<RwLock<HashMap<PageKey, i32>>>, // track page usage ()
    capacity: usize,                               // not used yet
    eviction_thread: Option<JoinHandle<()>>,
}

impl BufPool {
    pub fn new() -> Self {
        BufPool {
            page_pool: Arc::new(RwLock::new(HashMap::new())),
            page_id_queue: Arc::new(RwLock::new(VecDeque::new())),
            path: Arc::new(RwLock::new(HashMap::new())),
            usage_map: Arc::new(RwLock::new(HashMap::new())),
            capacity: BUFFER_CAPACITY,
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
        usage_map: Arc<RwLock<HashMap<PageKey, i32>>>,
        path_map: Arc<RwLock<HashMap<usize, PathBuf>>>,
    ) {
        let mut eviction_map: HashMap<PageKey, u32> = HashMap::new();

        loop {
            thread::sleep(Duration::from_secs(EVICTION_PERIOD as u64));
            println!("[Eviction] Eviction cycle started");
            {
                let mut usage_map_mutable_lock = usage_map.write().unwrap(); // prevents writes to it.
                let mut queue_mutable_lock = page_id_queue.write().unwrap(); // prevents read or writes to it.
                let size_of_queue = queue_mutable_lock.len();
                println!(
                    "[Eviction] Start processing on queue of size {}",
                    size_of_queue
                );
                for i in 0..size_of_queue {
                    let front_id_in_queue = queue_mutable_lock.pop_back().unwrap();

                    // check eviction logic
                    // if it is not used now -> then warn it, it num of warns is greater than a specific value then evict it
                    if !eviction_map.contains_key(&front_id_in_queue) {
                        eviction_map.insert(front_id_in_queue, 0);
                    }
                    if usage_map_mutable_lock[&front_id_in_queue] == 0 {
                        let mut curr_eviction_counter = eviction_map[&front_id_in_queue];
                        curr_eviction_counter += 1;
                        if curr_eviction_counter >= EVICTION_THREASHOLD {
                            println!("[Eviction] Evicting PageKey: {:?}", front_id_in_queue);
                            eviction_map.remove(&front_id_in_queue); // remove from the single threaded eviction map
                            usage_map_mutable_lock.remove(&front_id_in_queue);
                            {
                                // aquire write lock on the page pool
                                let mut page_mutable_lock = page_pool.write().unwrap();
                                let page = page_mutable_lock.remove(&front_id_in_queue).unwrap();
                                let guard = page.read().unwrap();
                                if guard.metadata.is_dirty() {
                                    let path_map_lock = path_map.read().unwrap();
                                    let file_path =
                                        &path_map_lock[&front_id_in_queue.get_file_id()];
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
                            println!(
                                "[Eviction] Evicting PageKey: {:?} done successfully",
                                front_id_in_queue
                            );
                        } else {
                            let eviction_val = eviction_map.get_mut(&front_id_in_queue).unwrap();
                            *eviction_val = curr_eviction_counter;
                        }
                    } else {
                        println!(
                            "[Eviction] No need to Evict PageKey: {:?}",
                            front_id_in_queue
                        );
                        queue_mutable_lock.push_back(front_id_in_queue);
                        let eviction_val = eviction_map.get_mut(&front_id_in_queue).unwrap();
                        *eviction_val = 0;
                    }
                }
            }
            println!("[Eviction] Eviction cycle ended");
        }
    }

    pub fn acquire_page(&self, req: &DiskRequest) -> anyhow::Result<LockGuard> {
        let mut usage_map_mutable_lock = self.usage_map.write().unwrap(); // prevents writes to it.
        {
            let page: Arc<RwLock<PageFrame>> = {
                let mut pool_read = self.page_pool.read().unwrap();
                let arc = if pool_read.contains_key(&req.page_key) {
                    let page_frame = pool_read.get(&req.page_key).unwrap();
                    let mut page_usage = usage_map_mutable_lock.get_mut(&req.page_key).unwrap();
                    *page_usage += 1;
                    Arc::clone(page_frame)
                } else {
                    // todo!();

                    // analyze file metadata to check if we can get it or not
                    // add it to the eviction queue
                    // then retrieve it if we can
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
                        );
                    }

                    drop(pool_read);
                    let mut pool_write = self.page_pool.write().unwrap();
                    pool_write.insert(req.page_key, Arc::clone(&page_arc));
                    let mut queue_lock = self.page_id_queue.write().unwrap();
                    queue_lock.push_front(req.page_key);
                    usage_map_mutable_lock.insert(req.page_key, 1);
                    page_arc
                };
                arc
            };

            match req.operation {
                Op::READ => {
                    let arc_clone = Arc::clone(&page);
                    let guard = unsafe { std::mem::transmute(page.read().unwrap()) };
                    println!(
                        "[BufPool] Acquired READ lock for PageKey: {:?}",
                        req.page_key
                    );
                    Ok(LockGuard::READ(guard, arc_clone, req.page_key))
                }
                Op::WRITE => {
                    let arc_clone = Arc::clone(&page);
                    let guard = unsafe { std::mem::transmute(page.write().unwrap()) };
                    println!(
                        "[BufPool] Acquired WRITE lock for PageKey: {:?}",
                        req.page_key
                    );
                    Ok(LockGuard::WRITE(guard, arc_clone, req.page_key))
                }
            }
        }
    }

    pub fn release_page(&self, guard: LockGuard) {
        let mut usage_map_mutable_lock = self.usage_map.write().unwrap(); // prevents writes to it.
        match guard {
            LockGuard::READ(_, _, key) => {
                let mut page_usage = usage_map_mutable_lock.get_mut(&key).unwrap();
                *page_usage -= 1;
                println!("[BufPool] Released READ lock for PageKey: {:?}", key);
            }
            LockGuard::WRITE(mut write_lock, _, key) => {
                write_lock.metadata.make_dirty();
                let mut page_usage = usage_map_mutable_lock.get_mut(&key).unwrap();
                *page_usage -= 1;
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

    // pub fn write_page(&self, req: &DiskRequest) {
    //     let file_path = &self.path[&req.object_id];
    //     let page_arc = Arc::clone(&self.map[&(req.object_id, req.page_id)]);
    //     let guard = page_arc.read().unwrap();
    //     DiskManager::write(file_path, req.page_id, &guard);
    // }

    // pub fn add_file(& mut self, file_path: &str, file_id: usize) {
    //     self.path.insert(file_id, PathBuf::from(file_path));
    // }

    // pub fn get_file(&self, file_id: usize) -> &Path {
    //     if self.path.contains_key(&file_id) {
    //          return  &self.path[&file_id];
    //     }
    //     &Path::new("s")
    // }
}

impl Drop for BufPool {
    fn drop(&mut self) {
        // flush all the dirty pages.
        if let Some(eviction_thread) = self.eviction_thread.take() {
            // take is used because of the JoinHandle doesn't implement the eviction thread
            eviction_thread.join();
        }
    }
}
