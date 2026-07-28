use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelTp {
    Table,
    Index(IdxTp),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rel {
    pub nm: String,
    pub id: u32,
    pub cols: Vec<Col>,
    pub tp: RelTp,
    pub fl_nm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Col {
    pub nm: String,
    pub dtp: Dtp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IdxTp {
    BPTree,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Dtp {
    Integer,
    Float,
    String,
    Boolean,
    Date,
}
