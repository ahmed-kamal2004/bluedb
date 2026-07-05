use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub name: String,
    pub tables: Vec<Table>,
    pub state: String, // path to entry data in the storage engine
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let encoded = std::fs::read_to_string(path)?;
        let bytes = base64::decode(encoded)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let json = String::from_utf8(bytes)?;

        let decoded: Database = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(decoded)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub state: String, // path to entry data in the storage engine
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub columns: Vec<Column>,
    pub index_type: IndexType,
    pub state: String, // path to entry data in the storage engine
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Integer,
    Float,
    String,
    Boolean,
    Date,
    Timestamp,
}
