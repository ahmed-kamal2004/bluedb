pub mod clog;
pub mod manager;

use anyhow::Result;
use std::sync::{Arc, RwLock};
use tracing::info;

use crate::lock::lock::{LockManager, LockType};

#[derive(Debug, PartialEq, Eq)]
pub enum TxnSt {
    Committed,
    Aborted,
    Active,
}

#[derive(Debug)]
pub struct TransactionInner {
    pub state: TxnSt,
    pub locks: Vec<(String, LockType)>,
}

#[derive(Debug)]
pub struct Transaction {
    pub txn_id: u64,
    pub lock_mngr: Arc<LockManager>,
    pub inner: RwLock<TransactionInner>,
}

impl Transaction {
    pub fn new(txn_id: u64, lock_mngr: Arc<LockManager>) -> Self {
        Transaction {
            txn_id,
            lock_mngr,
            inner: RwLock::new(TransactionInner {
                state: TxnSt::Active,
                locks: Vec::new(),
            }),
        }
    }

    /* Lock management */
    pub fn acquire_lock(&self, rsrc_id: &str, lk_tp: LockType) -> Result<()> {
        self.lock_mngr.acquire_lock(rsrc_id, self.txn_id, lk_tp)?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("Transaction {} lock poisoned", self.txn_id))?;
        guard.locks.push((rsrc_id.to_string(), lk_tp));
        Ok(())
    }

    pub fn release_lock(&self, rsrc_id: &str) -> Result<()> {
        self.lock_mngr.release_lock(rsrc_id, self.txn_id)?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("Transaction {} lock poisoned", self.txn_id))?;
        guard.locks.retain(|(id, _)| id != rsrc_id);
        Ok(())
    }

    pub fn release_all_locks(&self) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("Transaction {} lock poisoned", self.txn_id))?;
        for (rsrc_id, _) in &guard.locks {
            self.lock_mngr.release_lock(rsrc_id, self.txn_id)?;
        }

        guard.locks.clear();
        Ok(())
    }

    /* State management */
    pub fn commit(&self) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("Transaction {} lock poisoned", self.txn_id))?;
        guard.state = TxnSt::Committed;
        info!("Transaction {} committed successfully.", self.txn_id);
        Ok(())
    }

    pub fn abort(&self) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("Transaction {} lock poisoned", self.txn_id))?;
        guard.state = TxnSt::Aborted;
        info!("Transaction {} aborted successfully.", self.txn_id);
        Ok(())
    }
}
