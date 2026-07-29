use std::fs::File;

use super::super::constants::CATALOG_FILE_NAME;
use super::super::file::FileManager;
use crate::catalog::{self, structs::Rel};
use anyhow::Result;
use tracing::info;

pub struct StorageManager {
    db_pth: String,
}

impl StorageManager {
    pub fn new(db_pth: String) -> Self {
        StorageManager { db_pth }
    }

    pub fn get_db_path(&self) -> &str {
        &self.db_pth
    }

    /* Catalog operations */
    /* TODO: optimize catalog file management and layout, using JSONs is not efficient */
    pub fn load_catalog(&self) -> Result<Vec<Rel>> {
        let catalog_fl_pth = format!("{}/{}", self.db_pth, CATALOG_FILE_NAME);

        if FileManager::is_file_exists(&catalog_fl_pth) {
            info!(
                "StorageManager: Catalog file {} exists, loading catalog.",
                catalog_fl_pth
            );
        } else {
            info!(
                "StorageManager: Catalog file {} does not exist, creating new catalog file.",
                catalog_fl_pth
            );
            FileManager::create_file(&catalog_fl_pth)?;
            return Ok(Vec::new());
        }

        let catalog_data = match FileManager::read_whole_file(&catalog_fl_pth) {
            Ok(data) => data,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to read catalog file: {:?}", e));
            }
        };

        if catalog_data.is_empty() {
            return Ok(Vec::new());
        }

        let catalog = serde_json::de::from_slice(&catalog_data)?;
        Ok(catalog)
    }

    pub fn create_rel(&self, rel: Rel) -> Result<bool> {
        let catalog_fl_pth = format!("{}/{}", self.db_pth, CATALOG_FILE_NAME);
        let mut rels = self.load_catalog()?;
        rels.push(rel);
        let data = serde_json::ser::to_vec(&rels)?;
        FileManager::write_whole_file(&catalog_fl_pth, data)?;
        Ok(true)
    }

    pub fn delete_rel(&self, rel_name: &str) -> Result<()> {
        let catalog_fl_pth = format!("{}/{}", self.db_pth, CATALOG_FILE_NAME);
        let mut catalog = self.load_catalog()?;
        if let Some(pos) = catalog.iter().position(|rel| rel.nm == rel_name) {
            catalog.remove(pos);
            let data = serde_json::ser::to_vec(&catalog)?;
            FileManager::write_whole_file(&catalog_fl_pth, data)?;
        };
        Ok(())
    }
}
