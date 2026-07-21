use crate::{catalog::CATALOG_FILE, config::config::Config, engine::error::StartUpError};
use std::sync::{Arc, RwLock};
use super::structs::{Rel, RelTp};
pub struct Catalog {
    rels: Vec<Rel>,
}

impl Catalog {
    pub fn new(config: &Config) -> Result<Self, StartUpError> {
        // check if the main catalog path exists, if not throw error
        if !std::path::Path::new(&config.main_db_path).exists() {
            return Err(StartUpError::CatalogError(format!(
                "Main path {} does not exist",
                config.main_db_path
            )));
        }

        let ctlg_fl = std::path::Path::new(&config.main_db_path).join(CATALOG_FILE);
        //         let rels = if !ctlg_fl.exists() {
        //     std::fs::File::create(ctlg_fl).map_err(|e| StartUpError::CatalogError(e.to_string()))?;
        //     Vec::new()
        // } else {
        //     // Read it from disk
        // }

        let rels = Vec::new(); // Placeholder for now, will be loaded from disk in the future


        Ok(Catalog { rels })
    }
}
