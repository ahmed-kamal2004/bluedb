use anyhow::Result;
use sqlparser::{ast::Statement, dialect::GenericDialect, parser::Parser};
use std::sync::Arc;
use super::super::lock::lock::LockType;

use crate::{catalog::Catalog, txn::Transaction};

pub struct Validator {}

impl Validator {
    pub fn initial_validation_of_query(query: &str) -> Result<Statement> {
        let dialect = GenericDialect {};
        let mut ast = Parser::parse_sql(&dialect, query);
        match ast {
            Ok(ast) => {
                if ast.len() == 0 {
                    return Err(anyhow::anyhow!("Parser: Empty SQL query"));
                }
                if ast.len() > 1 {
                    return Err(anyhow::anyhow!(
                        "Parser: Multiple statements in the SQL query are not supported"
                    ));
                }
                for stat in &ast {
                    match stat {
                        Statement::Query(_) => {}
                        Statement::Insert { .. } => {}
                        Statement::Update { .. } => {}
                        Statement::Delete { .. } => {}
                        Statement::StartTransaction { .. } => {}
                        Statement::Rollback { .. } => {}
                        Statement::Commit { .. } => {}
                        Statement::CreateTable { .. } => {}
                        Statement::Drop { .. } => {}
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Parser: Unsupported statements in the SQL Query"
                            ));
                        }
                    }
                }
                return Ok(ast
                    .get(0)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Parser: Empty SQL query"))?);
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Parser: Failed to parse SQL query: {}", e));
            }
        }
    }

    pub fn validate_query_against_catalog(ast: &Statement, catalog: Arc<Catalog>, txn: Arc<Transaction>) -> Result<()> {
        match ast {
            Statement::CreateTable(crt_tbl_struct) => {
                Validator::validate_create_table_statement(crt_tbl_struct, catalog.clone(), txn)?;
            }
            Statement::Drop {
                object_type,
                if_exists,
                names,
                ..
            } => {
                Validator::validate_drop_table_statement(
                    object_type,
                    names,
                    *if_exists,
                    catalog.clone(),
                    txn.clone(),
                )?;
            }
            _ => {
                // For other statements, we can add more validation logic as needed
            }
        }

        Ok(())
    }

    /*Internal validation methods*/
    fn validate_create_table_statement(
        crt_tbl_struct: &sqlparser::ast::CreateTable,
        catalog: Arc<Catalog>,
        txn: Arc<Transaction>,
    ) -> Result<()> {
        let tbl_nm = crt_tbl_struct.name.to_string();

        // lock is aquired before checking if the table exists in the catalog, to avoid race conditions
        txn.acquire_lock(&tbl_nm, LockType::Exclusive)?;

        // check
        if catalog.rel_exists_by_name(&tbl_nm) && crt_tbl_struct.if_not_exists == false {
            return Err(anyhow::anyhow!(
                "Binder: Relation {} already exists in the catalog",
                tbl_nm
            ));
        }
        Ok(())
    }

    fn validate_drop_table_statement(
        object_type: &sqlparser::ast::ObjectType,
        names: &Vec<sqlparser::ast::ObjectName>,
        if_exists: bool,
        catalog: Arc<Catalog>,
        txn: Arc<Transaction>,
    ) -> Result<()> {
        if let sqlparser::ast::ObjectType::Table = object_type {
            for rel_name in names {
                let rel_name = rel_name.to_string();
                // just acquire a write lock over it, before checking if it exists in the catalog, to avoid race conditions
                txn.acquire_lock(&rel_name, LockType::Exclusive)?;

                // check
                if !catalog.rel_exists_by_name(&rel_name) && if_exists == false {
                    return Err(anyhow::anyhow!(
                        "Binder: Relation {} does not exist in the catalog",
                        rel_name
                    ));
                }
            }
        }
        Ok(())
    }
}
