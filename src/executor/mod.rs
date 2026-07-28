use crate::catalog::Catalog;

use super::result::QueryResult;
use super::storage::storage::StorageManager;
use crate::catalog::structs::Rel;
use anyhow::Result;
use sqlparser::ast::Statement;
use std::sync::Arc;
use tracing::info;
use utilis::map_crt_tbl_strct_to_rel;

mod utilis;

pub struct Executor {
    storage_manager: Arc<StorageManager>,
    catalog: Arc<Catalog>,
}

impl Executor {
    pub fn new(storage_manager: Arc<StorageManager>, catalog: Arc<Catalog>) -> Self {
        Executor {
            storage_manager,
            catalog,
        }
    }

    pub fn execute_query(&self, query: &Statement) -> Result<QueryResult> {
        match query {
            Statement::CreateTable(crt_tbl_strct) => self.execute_create_table(crt_tbl_strct),
            Statement::Drop {
                object_type,
                if_exists,
                names,
                cascade,
                restrict,
                purge,
                temporary,
                table,
            } => self.execute_drop(object_type, names, *if_exists),
            _ => Ok(QueryResult::Error {
                message: "Unsupported statement type for execution.".to_string(),
            }),
        }
    }

    fn execute_create_table(
        &self,
        crt_tbl_strct: &sqlparser::ast::CreateTable,
    ) -> Result<QueryResult> {
        let rel_name = crt_tbl_strct.name.to_string();
        if self.catalog.rel_exists_by_name(&rel_name) && !crt_tbl_strct.if_not_exists {
            return Ok(QueryResult::Error {
                message: format!("Table '{}' already exists.", rel_name),
            });
        }

        self.catalog
            .delete_rel_by_name(&rel_name, self.storage_manager.clone())?;

        let rel = map_crt_tbl_strct_to_rel(crt_tbl_strct);
        self.catalog
            .create_table(self.storage_manager.clone(), rel)?;
        Ok(QueryResult::Ddl {
            message: format!("Table '{}' created successfully.", rel_name),
        })
    }

    fn execute_drop(
        &self,
        object_type: &sqlparser::ast::ObjectType,
        names: &Vec<sqlparser::ast::ObjectName>,
        if_exists: bool,
    ) -> Result<QueryResult> {
        if let sqlparser::ast::ObjectType::Table = object_type {
            for rel_name in names {
                let rel_name = rel_name.to_string();
                if !self.catalog.rel_exists_by_name(&rel_name) && !if_exists {
                    return Ok(QueryResult::Error {
                        message: format!("Table '{}' does not exist.", rel_name),
                    });
                }

                self.catalog
                    .delete_rel_by_name(&rel_name, self.storage_manager.clone())?;

                info!("Table '{}' dropped successfully.", rel_name);
            }
            Ok(QueryResult::Ddl {
                message: "Tables dropped successfully.".to_string(),
            })
        } else {
            Ok(QueryResult::Error {
                message: "Only table drops are supported.".to_string(),
            })
        }
    }
}
