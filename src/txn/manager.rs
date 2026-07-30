use super::Transaction;
use super::clog::ClogManager;
use crate::catalog::Catalog;
use crate::control::ControlManager;
use crate::lock::lock::LockManager;
use crate::result::QueryResult;
use crate::txn;
use anyhow::Result;
use sqlparser::ast::Statement;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

pub struct TransactionManager {
    pub lock_mngr: Arc<LockManager>, // lock handling
    pub transactions: RwLock<HashMap<u64, Arc<Transaction>>>, // key: connection_id, value: transaction
    pub clog_mngr: Arc<ClogManager>, // in clog, we should be able to update the txn state, or get it
    pub control_mngr: Arc<ControlManager>, // control manager to generate next transaction ID
    pub ctlg: Arc<Catalog>,
}

impl TransactionManager {
    pub fn new(ctlg: Arc<Catalog>, control_mngr: Arc<ControlManager>) -> Self {
        let lock_mngr = Arc::new(LockManager::new());
        let clog_mngr = Arc::new(ClogManager::new());
        TransactionManager {
            lock_mngr,
            transactions: RwLock::new(HashMap::new()),
            clog_mngr,
            control_mngr,
            ctlg,
        }
    }

    /// Check if a statement is a transaction management request (BEGIN, COMMIT, ROLLBACK)
    pub fn is_txn_mngmnt_req(ast: &Statement) -> bool {
        // using the mysql, postgres way.
        match ast {
            Statement::StartTransaction { .. } => true,
            Statement::Commit { .. } => true,
            Statement::Rollback { .. } => true,
            _ => false,
        }
    }

    pub fn handle_txn_mngmnt_req(&self, ast: &Statement, conn_id: u64) -> Result<QueryResult> {
        let mut txn_guard = self.transactions.write().map_err(|_| {
            anyhow::anyhow!(
                "Failed to acquire transactions lock, connection id: {}",
                conn_id
            )
        })?;
        match ast {
            Statement::StartTransaction { .. } => {
                // first check if there is an already active transaction this connection.
                if txn_guard.contains_key(&conn_id) {
                    return Err(anyhow::anyhow!(
                        "Connection id: {} already has an active transaction",
                        conn_id
                    ));
                }

                // generate a new transaction ID using the control manager.
                let txn_id = self.control_mngr.generate_next_txn_id()?;

                // create a new transaction for this connection.
                let new_txn = Arc::new(Transaction::new(txn_id, self.lock_mngr.clone()));
                txn_guard.insert(conn_id, new_txn);
                Ok(QueryResult::StartTransaction {})
            }
            Statement::Commit { .. } => {
                // first check if there is an active transaction for this connection.
                let txn = txn_guard.get(&conn_id);

                match txn {
                    Some(txn) => {
                        // commit the transaction.
                        txn.commit()?;

                        // release all locks held by this transaction.
                        txn.release_all_locks()?;

                        // remove the transaction from the active transactions.
                        txn_guard.remove(&conn_id);
                        Ok(QueryResult::Commit {})
                    }
                    None => Err(anyhow::anyhow!(
                        "Connection id: {} does not have an active transaction to commit",
                        conn_id
                    )),
                }
            }
            Statement::Rollback { .. } => {
                // first check if there is an active transaction for this connection.
                let txn = txn_guard.get(&conn_id);

                match txn {
                    Some(txn) => {
                        // rollback the transaction.
                        txn.abort()?;

                        // release all locks held by this transaction.
                        txn.release_all_locks()?;

                        // remove the transaction from the active transactions.
                        txn_guard.remove(&conn_id);
                        Ok(QueryResult::Rollback {})
                    }
                    None => Err(anyhow::anyhow!(
                        "Connection id: {} does not have an active transaction to rollback",
                        conn_id
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("Not a transaction management request")),
        }
    }

    pub fn is_txn_active(&self, conn_id: u64) -> Result<bool> {
        let txn_guard = self.transactions.read().map_err(|_| {
            anyhow::anyhow!(
                "Failed to acquire transactions lock, connection id: {}",
                conn_id
            )
        })?;
        Ok(txn_guard.contains_key(&conn_id))
    }

    pub fn create_temp_txn_for_query_if_non_active_else_get_current(
        &self,
        conn_id: u64,
    ) -> Result<(bool, Arc<Transaction>)> {
        let mut txn_guard = self.transactions.write().map_err(|_| {
            anyhow::anyhow!(
                "Failed to acquire transactions lock, connection id: {}",
                conn_id
            )
        })?;

        // check if the connection has an active transaction.
        if let Some(txn) = txn_guard.get(&conn_id) {
            return Ok((false, txn.clone())); // the connection has an active transaction, return it.
        }

        // generate a new transaction ID using the control manager.
        let txn_id = self.control_mngr.generate_next_txn_id()?;

        // create a new transaction for this connection.
        let new_txn = Arc::new(Transaction::new(txn_id, self.lock_mngr.clone()));
        txn_guard.insert(conn_id, new_txn.clone());
        Ok((true, new_txn))
    }

    pub fn remove_temp_txn_if_created_for_query(
        &self,
        created_temp_txn: bool,
        conn_id: u64,
        is_commit: bool,
    ) -> Result<()> {
        if created_temp_txn {
            let mut txn_guard = self.transactions.write().map_err(|_| {
                anyhow::anyhow!(
                    "Failed to acquire transactions lock, connection id: {}",
                    conn_id
                )
            })?;

            let txn = txn_guard.get(&conn_id);
            match txn {
                Some(txn) => {
                    if is_commit {
                        txn.commit()?; // commit the transaction.
                    } else {
                        txn.abort()?; // abort the transaction.
                    }
                    // release all locks held by this transaction.
                    txn.release_all_locks()?;
                    txn_guard.remove(&conn_id);
                }
                None => {}
            }
        }
        Ok(())
    }
}
