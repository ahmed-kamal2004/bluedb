use sqlparser::{ast::Statement, dialect::GenericDialect, parser::Parser};

use crate::engine::error::ProcessingError;

pub fn parse_query(query: &str) -> Result<Vec<Statement>, ProcessingError> {
    let dialect = GenericDialect {};
    let ast = Parser::parse_sql(&dialect, query);
    match ast {
        Ok(ast) => {
            if ast.len() == 0 {
                return Err(ProcessingError::ParseError("Empty SQL query".to_string()));
            }
            if ast.len() > 1 {
                return Err(ProcessingError::ParseError(
                    "Multiple statements in the SQL query are not supported".to_string(),
                ));
            }
            for stat in &ast {
                match stat {
                    Statement::Query(_) => {}
                    Statement::CreateTable { .. } => {}
                    Statement::Insert { .. } => {}
                    Statement::Update { .. } => {}
                    Statement::Delete { .. } => {}
                    Statement::StartTransaction { .. } => {}
                    Statement::Commit { .. } => {}
                    // Statement::Drop { .. } => {},
                    _ => {
                        return Err(ProcessingError::ParseError(
                            "Unsupported statements in the SQL Query".to_string(),
                        ));
                    }
                }
            }
            return Ok(ast);
        }
        Err(e) => {
            return Err(ProcessingError::ParseError(format!(
                "Failed to parse SQL query: {}",
                e
            )));
        }
    }
}
