use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryResult {
    /// DDL -> Like CREATE TABLE, DROP TABLE.
    Ddl {
        message: String,
    },
    Error {
        message: String,
    },
}

impl Display for QueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryResult::Ddl { message } => write!(f, "DDL Result: {}", message),
            QueryResult::Error { message } => write!(f, "Error: {}", message),
        }
    }
}
