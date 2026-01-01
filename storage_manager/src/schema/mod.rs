mod types;
pub use types::Type;

#[repr(C)]
#[derive(Clone, Debug)]
pub(super) struct Schema {
    pub(super) sch_name: String,
    pub(super) sch_id: i32,
    pub(super) columns: Vec<Column>,
}

#[derive(Clone, Debug)]
pub(super) struct Column {
    col_name: String,
    col_id: i32,
    col_type: Type, // column size in bytes
}

impl PartialEq for Schema {
    fn eq(&self, other: &Self) -> bool {
        self.sch_id == other.sch_id
    }
}

// impl Schema {

//     //
// }
