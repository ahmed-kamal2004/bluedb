use tracing::{error, info};

use super::super::executor::Executor;
use super::validator::Validator;
use super::{super::catalog::Catalog, super::storage::storage::StorageManager};
use crate::config::config::Config;
use crate::result::QueryResult;
use anyhow::Result;
use std::sync::Arc;

pub struct Engine {
    config: Arc<Config>,
    executor: Option<Arc<Executor>>,
    catalog: Option<Arc<Catalog>>,
    storage_manager: Option<Arc<StorageManager>>,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        Engine {
            config,
            executor: None,
            catalog: None,
            storage_manager: None,
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
            Ok(_) => {
                self.catalog = Some(Arc::new(catalog));
                info!(
                    "Catalog loaded successfully with {} relations.",
                    self.catalog.as_ref().unwrap().rels.read().unwrap().len()
                );
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to load catalog: {:?}", e));
            }
        }

        // Initialize the executor
        let executor = Executor::new(storage_manager.clone());
        self.executor = Some(Arc::new(executor));
        info!("Executor initialized successfully.");

        Ok(())
    }

    pub fn process_query(&self, query: &str) -> Result<QueryResult> {
        // (0) Zero step is to check if the query can be processed without need to parse it, like ('\l' or '\dt' or '\d database_name') to list the databases and tables in the catalog
        // (1) First step to parse the query
        // (2) Second step to check the query data (binder stage)
        // (3) Third step to transform this AST into a logical plan (planner stage)
        // (4) Fourth step to transform the logical plan into a physical plan (optimizer stage)
        // (5) Fifth step to execute the physical plan and return the result (executor stage)
        // (6) Finally, return the result to the user

        // (Parser & Filter) Initial validation of the support scope
        let ast = Validator::initial_validation_of_query(query)?;

        // (Binder) Validation against the catalog
        Validator::validate_query_against_catalog(&ast, self.catalog.as_ref().unwrap().clone())?;

        // (Executor) Execute the query.
        let result = self.executor.as_ref().unwrap().execute_query(&ast)?;

        Ok(result)
    }
}
