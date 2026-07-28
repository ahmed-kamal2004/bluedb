use crate::catalog::structs::{Rel, RelTp::Table};
use sqlparser::ast::CreateTable;
pub fn map_crt_tbl_strct_to_rel(crt_tbl_strct: &sqlparser::ast::CreateTable) -> Rel {
    Rel {
        nm: crt_tbl_strct.name.to_string(),
        id: 0, // This should be generated or fetched from somewhere
        cols: crt_tbl_strct
            .columns
            .iter()
            .map(|col| crate::catalog::structs::Col {
                nm: col.name.to_string(),
                dtp: match &col.data_type {
                    sqlparser::ast::DataType::Int(_) => crate::catalog::structs::Dtp::Integer,
                    sqlparser::ast::DataType::Float(_) => crate::catalog::structs::Dtp::Float,
                    sqlparser::ast::DataType::Varchar(_) => crate::catalog::structs::Dtp::String,
                    sqlparser::ast::DataType::Boolean => crate::catalog::structs::Dtp::Boolean,
                    sqlparser::ast::DataType::Date => crate::catalog::structs::Dtp::Date,
                    _ => unimplemented!("Data type not supported yet"),
                },
            })
            .collect(),
        tp: Table,             // Set appropriate type if needed
        fl_nm: "".to_string(), // Set appropriate file name if needed
    }
}
