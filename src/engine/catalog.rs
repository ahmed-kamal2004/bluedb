use crate::{config::config::Config, engine::error::StartUpError};

use super::structure::structs::{Column, DataType, Database, Table};

pub struct Catalog {
    databases: Vec<Database>,
}

impl Catalog {
    pub fn new(config: &Config) -> Result<Self, StartUpError> {
        // create a dummy databases for testing purposes
        let new_database = Database {
            name: "test_db".to_string(),
            tables: vec![Table {
                name: "test_table".to_string(),
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                    },
                    Column {
                        name: "name".to_string(),
                        data_type: DataType::String,
                    },
                ],
                state: "test_table_state".to_string(),
            }],
            state: "test_db_state".to_string(),
        };

        // check if the main database path exists, if not create it
        if !std::path::Path::new(&config.main_db_path).exists() {
            std::fs::create_dir_all(&config.main_db_path)
                .map_err(|e| StartUpError::CatalogError(e.to_string()))?;
        }

        // get the databases folder
        let database_folder = std::path::Path::new(&config.main_db_path).join("databases");
        if !database_folder.exists() {
            std::fs::create_dir_all(&database_folder)
                .map_err(|e| StartUpError::CatalogError(e.to_string()))?;
            return Ok(Catalog {
                databases: Vec::new(),
            });
        }

        let mut databases = Vec::new();
        for entry in std::fs::read_dir(&database_folder)
            .map_err(|e| StartUpError::CatalogError(e.to_string()))?
        {
            let entry = entry.map_err(|e| StartUpError::CatalogError(e.to_string()))?;
            let path = entry.path();
            if path.is_file() {
                match Database::new(path.to_str().unwrap()) {
                    Ok(db) => databases.push(db),
                    Err(e) => return Err(StartUpError::CatalogError(e.to_string())),
                }
            }
        }

        databases.push(new_database);
        Ok(Catalog { databases })
    }

    pub fn contains_database(&self, db_name: &str) -> bool {
        self.databases.iter().any(|db| db.name == db_name)
    }

    pub fn list_databases(&self) -> Vec<Database> {
        self.databases.clone()
    }

    pub fn list_tables(&self, db_name: &str) -> Vec<super::structure::structs::Table> {
        if let Some(db) = self.databases.iter().find(|db| db.name == db_name) {
            db.tables.clone()
        } else {
            Vec::new()
        }
    }
}
