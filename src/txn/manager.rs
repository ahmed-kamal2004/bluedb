use super::Transaction;
use super::clog::ClogManager;
use crate::catalog::Catalog;
use crate::lock::lock::LockManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

pub struct TransactionManager {
    pub lock_mngr: Arc<LockManager>,
    pub transactions: RwLock<HashMap<u64, Arc<Transaction>>>, // key: connection_id, value: transaction
    pub clog_mngr: Arc<ClogManager>,
    pub ctlg: Arc<Catalog>,
}
