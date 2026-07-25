pub mod txn;
pub mod clog;

use std::sync::{Arc, RwLock};

#[derive(Debug, PartialEq, Eq)]
pub enum TxnSt {
    Committed,
    Aborted,
    Active,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum IsolLvl {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
}

#[derive(Debug)]
pub struct Transaction {
    pub txn_id: u32,
    pub state: Arc<RwLock<TxnSt>>,
    pub isol_lvl: IsolLvl,
}