use crate::engine::error::ProcessingError;

use super::parser::parse::parse_query;
pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Engine
    }

    pub fn process_query(&self, query: &str) -> Result<Vec<u8>, ProcessingError> {
        // (1) First step to parse the query
        // (2) Second step to check the query data (binder stage)
        // (3) Third step to transform this AST into a logical plan (planner stage)
        // (4) Fourth step to transform the logical plan into a physical plan (optimizer stage)
        // (5) Fifth step to execute the physical plan and return the result (executor stage)
        // (6) Finally, return the result to the user

        // Parse
        let ast = parse_query(query)?;

        // Binder, Validate againest the catalog and check the query is valid or not (data types check)

        Ok(Vec::new())
    }
}
