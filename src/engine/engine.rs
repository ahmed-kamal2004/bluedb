use super::super::control::ControlManager;
use tracing::info;

use super::super::executor::Executor;
use super::validator::Validator;
use super::{super::catalog::Catalog, super::storage::storage::StorageManager};
use crate::config::config::Config;
use crate::pool::pool::BufPl;
use crate::result::QueryResult;
use crate::txn::manager::TransactionManager;
use crate::txn::{self, Transaction};
use anyhow::Result;
use std::sync::Arc;

pub struct Engine {
    config: Arc<Config>,
    executor: Arc<Executor>,
    catalog: Arc<Catalog>,
    storage_manager: Arc<StorageManager>,
    control_mngr: Arc<ControlManager>,
    buf_pool: Arc<BufPl>,
    txn_mngr: Arc<TransactionManager>,
}

impl Engine {
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        info!("Initializing Engine with config: {:?}", config);
        // Initialize the storage manager
        let storage_manager = Arc::new(StorageManager::new(config.main_db_path.clone()));
        info!(
            "[1/5] Storage Manager initialized with DB path: {}",
            config.main_db_path
        );

        // Initialize the catalog
        let mut catalog = Catalog::new();
        match catalog.load(storage_manager.clone()) {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to load catalog: {:?}", e));
            }
        }

        let catalog = Arc::new(catalog);
        info!(
            "[2/5] Catalog loaded successfully with {} relations.",
            catalog.rels.read().unwrap().len()
        );

        // Initialize the executor
        let executor = Executor::new(storage_manager.clone(), catalog.clone());
        let executor = Arc::new(executor);
        info!("[3/5] Executor initialized successfully.");
        info!("[4/5] Control Manager initialized successfully.");

        // Intialize the buffer pool
        let buf_pl = Arc::new(BufPl::new(
            config.main_db_path.clone(),
            config.buffer_pool_size,
        ));
        info!(
            "[5/5] Buffer Pool initialized successfully with capacity: {}.",
            config.buffer_pool_size
        );

        let control_mngr = Arc::new(ControlManager::new(format!(
            "{}/{}",
            config.main_db_path,
            crate::constants::CONTROL_FILE_NAME
        ))?);

        Ok(Engine {
            config,
            executor,
            catalog: catalog.clone(),
            storage_manager,
            control_mngr: control_mngr.clone(),
            buf_pool: buf_pl.clone(),
            txn_mngr: Arc::new(TransactionManager::new(catalog, control_mngr)),
        })
    }

    pub fn process_query(&self, query: &str, conn_id: u64) -> Result<QueryResult> {
        // (Parser & Filter) Initial validation of the support scope
        if let Ok(ast) = Validator::initial_validation_of_query(query) {
            // first, check if the query is a transaction management request (BEGIN, COMMIT, ROLLBACK)
            // if it is, we will handle it separately, and not go through the normal query processing flow.
            // directly handle it in the transaction manager, and return the result.
            if TransactionManager::is_txn_mngmnt_req(&ast) {
                match self.txn_mngr.handle_txn_mngmnt_req(&ast, conn_id) {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        return Ok(QueryResult::Error {
                            message: e.to_string(),
                        });
                    }
                }
            }

            // if we are in the middle of a transaction
            // txn is used to acquire locks during the binding stage.
            let (nex_txn_flag, txn) = self
                .txn_mngr
                .create_temp_txn_for_query_if_non_active_else_get_current(conn_id)?;

            let outcome: anyhow::Result<QueryResult> = {
                // (Binder) Validation against the catalog, we acquire locks per resource at this stage. (first stage to see the resources needed for the query)
                Validator::validate_query_against_catalog(&ast, self.catalog.clone(), txn)?;

                // (Executor) Execute the query.
                let result = self.executor.execute_query(&ast)?;

                Ok(result)
            };

            let success = outcome.is_ok();

            // remove the temporary transaction if it was created for this query, after the query is processed.
            self.txn_mngr
                .remove_temp_txn_if_created_for_query(nex_txn_flag, conn_id, success)?;

            match outcome {
                Ok(result) => Ok(result),
                Err(e) => Ok(QueryResult::Error {
                    message: e.to_string(),
                }),
            }
        } else {
            Ok(QueryResult::Error {
                message: format!("Parser: Failed to parse SQL query: {}", query),
            })
        }
    }
}
