use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex, RwLock};

use anyhow::Result;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum LockType {
    None,
    IntentShared,
    Shared,
    IntentExclusive,
    Exclusive,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Lock {
    tp: LockType,
    txn_id: u64,
}

#[derive(Debug)]
pub struct ResourceLockInner {
    pub locks: HashSet<Lock>,
    pub highest_lock: LockType,
}

#[derive(Debug)]
pub struct ResourceLock {
    pub inner: Mutex<ResourceLockInner>,
    pub cvar: Condvar,
}

/// Holds the locks per resource.
#[derive(Debug)]
pub struct LockManager {
    locks: Arc<RwLock<HashMap<String, Arc<ResourceLock>>>>,
}

impl LockManager {
    pub fn new() -> Self {
        LockManager {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn acquire_lock(&self, rsrc_id: &str, txn_id: u64, lk_tp: LockType) -> Result<()> {
        loop {
            // outer guard to access the locks map
            let resource_lock = {
                let mut locks_guard = self.locks.write().unwrap();
                locks_guard
                    .entry(rsrc_id.to_string())
                    .or_insert_with(|| {
                        Arc::new(ResourceLock {
                            inner: Mutex::new(ResourceLockInner {
                                locks: HashSet::new(),
                                highest_lock: LockType::None,
                            }),
                            cvar: Condvar::new(),
                        })
                    })
                    .clone()
            };
            // inner guard to access the specific resource lock
            let mut inner_guard = resource_lock.inner.lock().unwrap();
            if compatible(&inner_guard.highest_lock, &lk_tp) {
                inner_guard.locks.insert(Lock { tp: lk_tp, txn_id });
                if lk_tp > inner_guard.highest_lock {
                    inner_guard.highest_lock = lk_tp;
                }
                break;
            } else {
                inner_guard = resource_lock.cvar.wait(inner_guard).unwrap();
            }
        }
        Ok(())
    }

    pub fn release_lock(&self, rsrc_id: &str, txn_id: u64) -> Result<()> {
        // acquire outer guard
        let ref resource_lock = {
            let locks_guard = self.locks.read().unwrap();
            match locks_guard.get(rsrc_id) {
                Some(lock) => lock.clone(),
                None => return Ok(()), // No locks for this resource
            }
        };

        // acquire inner guard
        let mut inner_guard = resource_lock.inner.lock().unwrap();
        inner_guard.locks.retain(|lock| lock.txn_id != txn_id);
        inner_guard.highest_lock = inner_guard
            .locks
            .iter()
            .map(|lock| lock.tp)
            .max()
            .unwrap_or(LockType::None);
        resource_lock.cvar.notify_all();
        Ok(())
    }
}

fn compatible(lk1: &LockType, lk2: &LockType) -> bool {
    match (lk1, lk2) {
        (LockType::Shared, LockType::Shared) => true,
        (LockType::Shared, LockType::IntentShared) => true,
        (LockType::IntentShared, LockType::Shared) => true,
        (LockType::IntentShared, LockType::IntentShared) => true,
        (LockType::None, _) => true,
        (_, LockType::None) => true,
        _ => false,
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_lock_compatibility() {
        assert!(compatible(&LockType::Shared, &LockType::Shared));
        assert!(compatible(&LockType::Shared, &LockType::IntentShared));
        assert!(compatible(&LockType::IntentShared, &LockType::Shared));
        assert!(compatible(&LockType::IntentShared, &LockType::IntentShared));
        assert!(!compatible(&LockType::Exclusive, &LockType::Shared));
        assert!(!compatible(&LockType::Exclusive, &LockType::IntentShared));
        assert!(!compatible(&LockType::Shared, &LockType::Exclusive));
        assert!(!compatible(&LockType::IntentShared, &LockType::Exclusive));
    }

    #[test]
    fn test_lock_acquire_release() {
        let lock_manager = LockManager::new();
        let rsrc_id = "resource1";
        let txn_id1 = 1;
        let txn_id2 = 2;

        // Acquire shared lock for txn_id1
        lock_manager
            .acquire_lock(rsrc_id, txn_id1, LockType::Shared)
            .unwrap();
        {
            let locks_guard = lock_manager.locks.read().unwrap();
            let resource_lock = locks_guard.get(rsrc_id).unwrap();
            let inner_guard = resource_lock.inner.lock().unwrap();
            assert_eq!(inner_guard.locks.len(), 1);
            assert_eq!(inner_guard.highest_lock, LockType::Shared);
        }

        // Acquire shared lock for txn_id2
        lock_manager
            .acquire_lock(rsrc_id, txn_id2, LockType::Shared)
            .unwrap();
        {
            let locks_guard = lock_manager.locks.read().unwrap();
            let resource_lock = locks_guard.get(rsrc_id).unwrap();
            let inner_guard = resource_lock.inner.lock().unwrap();
            assert_eq!(inner_guard.locks.len(), 2);
            assert_eq!(inner_guard.highest_lock, LockType::Shared);
        }

        // Release lock for txn_id1
        lock_manager.release_lock(rsrc_id, txn_id1).unwrap();
        {
            let locks_guard = lock_manager.locks.read().unwrap();
            let resource_lock = locks_guard.get(rsrc_id).unwrap();
            let inner_guard = resource_lock.inner.lock().unwrap();
            assert_eq!(inner_guard.locks.len(), 1);
            assert_eq!(inner_guard.highest_lock, LockType::Shared);
        }
    }

    #[test]
    fn test_lock_acquire_release_exclusive() {
        let lock_manager = Arc::new(LockManager::new());
        let rsrc_id = "resource2";
        let txn_id1 = 1;
        let txn_id2 = 2;

        // Acquire exclusive lock for txn_id1
        lock_manager
            .acquire_lock(rsrc_id, txn_id1, LockType::Exclusive)
            .unwrap();
        {
            let locks_guard = lock_manager.locks.read().unwrap();
            let resource_lock = locks_guard.get(rsrc_id).unwrap();
            let inner_guard = resource_lock.inner.lock().unwrap();
            assert_eq!(inner_guard.locks.len(), 1);
            assert_eq!(inner_guard.highest_lock, LockType::Exclusive);
        }

        // Attempt to acquire shared lock for txn_id2 (should block)
        let lock_manager_clone = lock_manager.clone();
        let handle = std::thread::spawn(move || {
            lock_manager_clone
                .acquire_lock(rsrc_id, txn_id2, LockType::Shared)
                .unwrap();
        });

        // Sleep for a short duration to ensure the thread is blocked
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Release exclusive lock for txn_id1
        lock_manager.release_lock(rsrc_id, txn_id1).unwrap();

        // Wait for the thread to finish
        handle.join().unwrap();

        {
            let locks_guard = lock_manager.locks.read().unwrap();
            let resource_lock = locks_guard.get(rsrc_id).unwrap();
            let inner_guard = resource_lock.inner.lock().unwrap();
            assert_eq!(inner_guard.locks.len(), 1);
            assert_eq!(inner_guard.highest_lock, LockType::Shared);
        }
    }
}
