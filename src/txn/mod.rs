use std::sync::{Arc, RwLock};

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
