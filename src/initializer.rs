use anyhow::Result;
use tracing::info;

/// This struct is responsible for file validations and initialization of the database files and directories.
/// Can be exteneded to include more initialization tasks in the future.
pub struct Initializer {}

impl Initializer {
    pub fn initialize_system(db_path: &str) -> Result<()> {
        // first check if the database directory exists, if not create it.
        if !crate::file::FileManager::is_directory_exists(db_path) {
            crate::file::FileManager::create_directory(db_path)?;
            info!("[1/4] Database directory {} created successfully.", db_path);
        } else {
            info!("[1/4] Database directory {} already exists.", db_path);
        }

        // [catalog] check if the catalog file exists, if not create it.
        let catalog_file_path = format!("{}/{}", db_path, crate::constants::CATALOG_FILE_NAME);
        if !crate::file::FileManager::is_file_exists(&catalog_file_path) {
            crate::file::FileManager::create_file(&catalog_file_path)?;
            info!(
                "[2/4] Catalog file {} created successfully.",
                catalog_file_path
            );
        } else {
            info!("[2/4] Catalog file {} already exists.", catalog_file_path);
        }

        // [control] check if the control file exists, if not create it.
        let control_file_path = format!("{}/{}", db_path, crate::constants::CONTROL_FILE_NAME);
        if !crate::file::FileManager::is_file_exists(&control_file_path) {
            crate::file::FileManager::create_file(&control_file_path)?;
            // initialize the next transaction ID to 1.
            let next_txn_id: u64 = 1;
            let next_txn_id_bytes = next_txn_id.to_le_bytes();
            crate::file::FileManager::write_to_file(
                &control_file_path,
                crate::constants::CONTROL_NEXT_TXN_ID_OFFSET as u64,
                next_txn_id_bytes.to_vec(),
                crate::constants::CONTROL_NEXT_TXN_ID_SIZE,
            )?;
            info!(
                "[3/4] Control file {} created successfully.",
                control_file_path
            );
        } else {
            info!("[3/4] Control file {} already exists.", control_file_path);
        }

        // [clog] check if the clog file exists, if not create it.
        let clog_file_path = format!("{}/{}", db_path, crate::constants::CLOG_FILE_NAME);
        if !crate::file::FileManager::is_file_exists(&clog_file_path) {
            crate::file::FileManager::create_file(&clog_file_path)?;
            info!("[4/4] Clog file {} created successfully.", clog_file_path);
        } else {
            info!("[4/4] Clog file {} already exists.", clog_file_path);
        }

        Ok(())
    }
}
