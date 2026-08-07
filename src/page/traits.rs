use crate::heap::heap::Row;
use anyhow::Result;
pub trait Initialize {
    fn initialize(&mut self, txn_id: u32) -> Result<()>;
}
