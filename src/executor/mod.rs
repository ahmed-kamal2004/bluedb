use super::result::QueryResult;
use anyhow::Result;
use sqlparser::ast::Statement;
use std::sync::Arc;

use super::storage::storage::StorageManager;
pub struct Executor {
    storage_manager: Arc<StorageManager>,
}

impl Executor {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Executor { storage_manager }
    }

    pub fn execute_query(&self, query: &Statement) -> Result<QueryResult> {
        // Placeholder implementation
        Ok(QueryResult::Ddl {
            message: "Query executed".to_string(),
        })
    }
}
