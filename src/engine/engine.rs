use tracing::info;

use super::super::executor::Executor;
use super::validator::Validator;
use super::{super::catalog::Catalog, super::storage::storage::StorageManager};
use crate::config::config::Config;
use crate::lock::lock::LockManager;
use crate::result::QueryResult;
use crate::txn::{self, Transaction};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

pub struct Engine {
    config: Arc<Config>,
    executor: Option<Arc<Executor>>,
    catalog: Option<Arc<Catalog>>,
    storage_manager: Option<Arc<StorageManager>>,
    transactions: RwLock<HashMap<u64, Arc<Transaction>>>, // key: connection_id, value: transaction
    lock_manager: Arc<LockManager>,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        Engine {
            config,
            executor: None,
            catalog: None,
            storage_manager: None,
            transactions: RwLock::new(HashMap::new()),
            lock_manager: Arc::new(LockManager::new()),
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing Engine with config: {:?}", self.config);
        // Initialize the storage manager
        let storage_manager = Arc::new(StorageManager::new(self.config.main_db_path.clone()));
        self.storage_manager = Some(storage_manager.clone());
        info!(
            "Storage Manager initialized with DB path: {}",
            self.config.main_db_path
        );

        // Initialize the catalog
        let mut catalog = Catalog::new();
        match catalog.load(storage_manager.clone()) {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to load catalog: {:?}", e));
            }
        }

        self.catalog = Some(Arc::new(catalog));
        info!(
            "Catalog loaded successfully with {} relations.",
            self.catalog.as_ref().unwrap().rels.read().unwrap().len()
        );

        // Initialize the executor
        let executor = Executor::new(
            storage_manager.clone(),
            self.catalog.as_ref().unwrap().clone(),
        );
        self.executor = Some(Arc::new(executor));
        info!("Executor initialized successfully.");

        Ok(())
    }

    pub fn process_query(&self, query: &str, conn_id: u64) -> Result<QueryResult> {
        // first, we need to check if the current connection has an active transaction.
        let mut txns_guard = self.transactions.write().map_err(|_| {
            anyhow::anyhow!(
                "Failed to acquire transactions lock, connection id: {}",
                conn_id
            )
        })?;
        // then check if the connection has an active one.
        let txn = txns_guard.get(&conn_id);
        let (created_temp_txn, txn) = match txn {
            Some(txn) => (false, txn.clone()), // the connection has an active transaction, we don't need to create a new one.
            None => {
                // the connection does not have an active transaction, we need to create a new one.
                let new_txn = Arc::new(Transaction::new(conn_id, self.lock_manager.clone()));
                txns_guard.insert(conn_id, new_txn.clone());
                (true, new_txn.clone()) // we created a new transaction for this connection.
            }
        };
        drop(txns_guard); // drop the lock guard to avoid deadlocks when executing the query.

        let outcome: anyhow::Result<QueryResult> = {
            // (Parser & Filter) Initial validation of the support scope
            let ast = Validator::initial_validation_of_query(query)?;

            // (Binder) Validation against the catalog, we acquire locks per resource at this stage. (first stage to see the resources needed for the query)
            let catalog = self
                .catalog
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Catalog is not initialized"))?;
            Validator::validate_query_against_catalog(&ast, catalog.clone())?;

            // (Executor) Execute the query.
            let result = self.executor.as_ref().unwrap().execute_query(&ast)?;

            Ok((result))
        };

        // if we created a temporary transaction for this connection, we need to remove it.
        if created_temp_txn {
            let mut txns_guard = self.transactions.write().map_err(|_| {
                anyhow::anyhow!(
                    "Failed to acquire transactions lock, connection id: {}",
                    conn_id
                )
            })?;
            txns_guard.remove(&conn_id);
            drop(txns_guard); // drop the lock guard after removing the temporary transaction.
        }

        let result = match outcome {
            Ok(result) => {
                if created_temp_txn {
                    txn.commit()?;
                    txn.release_all_locks()?;
                    info!("Transaction {} committed successfully.", conn_id);
                }
                result
            }
            Err(_) => {
                txn.abort()?;
                txn.release_all_locks()?;
                info!("Transaction {} aborted due to error.", conn_id);
                QueryResult::Error {
                    message: format!("Transaction {} aborted due to error.", conn_id),
                }
            }
        };

        Ok(result)
    }
}
