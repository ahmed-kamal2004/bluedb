use super::parser::parse::parse_query;
use crate::engine::catalog::Catalog;
use crate::{config::config::Config, engine::error::ProcessingError};
pub struct Engine {
    config: Config,
    catalog: Catalog,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        let catalog = Catalog::new(&config).expect("Failed to initialize catalog");
        Engine { config, catalog }
    }

    pub fn process_query(&self, query: &str, db_name: &str) -> Result<Vec<u8>, ProcessingError> {
        // (0) Zero step is to check if the query can be processed without need to parse it, like ('\l' or '\dt' or '\d database_name') to list the databases and tables in the catalog
        // (1) First step to parse the query
        // (2) Second step to check the query data (binder stage)
        // (3) Third step to transform this AST into a logical plan (planner stage)
        // (4) Fourth step to transform the logical plan into a physical plan (optimizer stage)
        // (5) Fifth step to execute the physical plan and return the result (executor stage)
        // (6) Finally, return the result to the user

        // (0)
        if query.eq("\\l") {
            let databases = self.catalog.list_databases();
            let mut result = Vec::new();
            for db in databases {
                result.push(db.name);
            }
            return Ok(result.join("\n").into_bytes());
        }

        if query.eq("\\dt") {
            let tables = self.catalog.list_tables(db_name);
            let mut result = Vec::new();
            for table in tables {
                result.push(table.name);
            }
            return Ok(result.join("\n").into_bytes());
        }

        // Parse
        let ast = parse_query(query)?;

        // Binder, Validate againest the catalog and check the query is valid or not (data types check)

        Ok(Vec::new())
    }

    pub fn contains_database(&self, db_name: &str) -> bool {
        self.catalog.contains_database(db_name)
    }
}
