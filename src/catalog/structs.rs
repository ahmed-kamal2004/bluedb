use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelTp {
    Table,
    Index(IdxTp),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rel {
    pub mn: String,
    pub cols: Vec<Col>,
    pub tp: RelTp,
    pub idxs: Vec<Rel>, // indexes on this relation
    pub fl_pth: String, // path to entry data in the storage engine
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
    Timestamp,
}

mod tests {
    use super::*;

    #[test]
    fn test_tbl_serialization() {
        let tbl = Rel {
            mn: "test_table".to_string(),
            cols: vec![
                Col {
                    nm: "id".to_string(),
                    dtp: Dtp::Integer,
                },
                Col {
                    nm: "name".to_string(),
                    dtp: Dtp::String,
                },
            ],
            tp: RelTp::Table,
            idxs: vec![
                Rel {
                    mn: "test_index".to_string(),
                    cols: vec![Col {
                        nm: "id".to_string(),
                        dtp: Dtp::Integer,
                    }],
                    tp: RelTp::Index(IdxTp::BPTree),
                    idxs: vec![],
                    fl_pth: "active".to_string(),
                },
            ],
            fl_pth: "active".to_string(),
        };

        let serialized = serde_json::to_string(&tbl).unwrap();
        let deserialized: Rel = serde_json::from_str(&serialized).unwrap();

        assert_eq!(tbl.mn, deserialized.mn);
        assert_eq!(tbl.cols.len(), deserialized.cols.len());
        assert_eq!(tbl.tp, deserialized.tp);
        assert_eq!(tbl.fl_pth, deserialized.fl_pth);
    }
}