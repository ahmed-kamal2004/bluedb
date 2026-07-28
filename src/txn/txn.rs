use super::IsolLvl;

/// Integration with the clog is a must.
pub fn tk_row(current_txn: u32, isol_lvl: IsolLvl, tmin: u32, tmax: u32) -> bool {
    if tmin == current_txn && tmax == 0 {
        // This is written by the current txn.
        return true;
    }

    // match isol_lvl {
    //      IsolLvl::ReadUncommitted => {
    //         // return all rows
    //      }
    //      IsolLvl::ReadCommitted => {
    //          // In Read Committed, a row is visible if it was committed before the current transaction started.
    //      }
    //      IsolLvl::RepeatableRead => {
    //          // In Repeatable Read, a row is visible if it was committed before the current transaction started and has not been modified by another transaction.
    //      }
    //  }
    return true;
}
