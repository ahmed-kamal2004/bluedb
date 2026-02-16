use std::collections::HashMap;

use super::table::TableMetaData;

pub struct Catalog {
    tables: HashMap<String, TableMetaData>,
}
