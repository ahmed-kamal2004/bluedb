use super::support::initial_validation_of_query;
use crate::{
    catalog::catalog::Catalog,
    config::config::Config,
    engine::error::ProcessingError,
};
pub struct Engine {
    config: Config,
    catalog: Catalog,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        let catalog = Catalog::new(&config).expect("Failed to initialize catalog");
        Engine { config, catalog }
    }

    pub fn process_query(&self, query: &str) -> Result<Vec<u8>, ProcessingError> {
        // (0) Zero step is to check if the query can be processed without need to parse it, like ('\l' or '\dt' or '\d database_name') to list the databases and tables in the catalog
        // (1) First step to parse the query
        // (2) Second step to check the query data (binder stage)
        // (3) Third step to transform this AST into a logical plan (planner stage)
        // (4) Fourth step to transform the logical plan into a physical plan (optimizer stage)
        // (5) Fifth step to execute the physical plan and return the result (executor stage)
        // (6) Finally, return the result to the user

        // Parse
        let ast = initial_validation_of_query(query)?;
        println!("Parsed AST: {:?}", ast);

        // Binder, Validate againest the catalog and check the query is valid or not (data types check)

        Ok(Vec::new())
    }
}
