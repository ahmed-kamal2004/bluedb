use anyhow::Result;
use std::sync::Mutex;
use tracing::info;

// Postgres handles the generation of the transaction ID using pg_control.

use crate::file::FileManager;

pub struct ControlManager {
    pub next_txn_id: Mutex<u64>, // the next transaction ID to be used
    pub control_file_path: String,
}

// The control manager is responsible for managing the control file, which contains metadata about the database, including the next transaction ID to be used.
// The next transaction ID is used to ensure that each transaction has a unique identifier.
// Disk operations are expensive, so we will keep the next transaction ID in memory, and only write it to disk after a fixed number of transactions have been committed.
// every 32 transactions.

impl ControlManager {
    pub fn new(control_file_path: String) -> Result<Self> {
        // first check if the control file exists, if not, create it and initialize the next transaction ID to 1.
        if !FileManager::is_file_exists(&control_file_path) {
            info!(
                "Control Manager: Control file {} does not exist.",
                control_file_path
            );
            return Err(anyhow::anyhow!(
                "Control Manager: Control file {} does not exist.",
                control_file_path
            ));
        } else {
            info!(
                "Control Manager: Control file {} exists.",
                control_file_path
            );

            // read the next transaction ID from the control file.
            let next_txn_id_bytes = FileManager::read_from_file(
                &control_file_path,
                crate::constants::CONTROL_NEXT_TXN_ID_OFFSET as u64,
                crate::constants::CONTROL_NEXT_TXN_ID_SIZE,
            )?;
            let next_txn_id = u64::from_le_bytes(next_txn_id_bytes.try_into().map_err(|_| {
                anyhow::anyhow!(
                    "Control Manager: Failed to read next transaction ID from control file {}",
                    control_file_path
                )
            })?);

            info!(
                "Control Manager: Next transaction ID read from control file {}: {}",
                control_file_path, next_txn_id
            );

            Ok(ControlManager {
                next_txn_id: Mutex::new(next_txn_id),
                control_file_path,
            })
        }
    }

    pub fn generate_next_txn_id(&self) -> Result<u64> {
        let mut next_txn_id = self.next_txn_id.lock().map_err(|_| {
            anyhow::anyhow!("Control Manager: Failed to acquire write lock for next transaction ID")
        })?;

        let current_txn_id = *next_txn_id;
        *next_txn_id += 1;

        // write the next transaction ID to the control file.
        FileManager::write_to_file(
            &self.control_file_path,
            crate::constants::CONTROL_NEXT_TXN_ID_OFFSET as u64,
            next_txn_id.to_le_bytes().to_vec(),
            crate::constants::CONTROL_NEXT_TXN_ID_SIZE,
        )?;

        Ok(current_txn_id)
    }
}
