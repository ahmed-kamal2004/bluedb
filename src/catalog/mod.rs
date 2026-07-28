pub mod structs;
use super::storage::storage::StorageManager;
use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use structs::Rel;

pub struct Catalog {
    pub rels: RwLock<HashMap<String, Rel>>,
}

impl Catalog {
    pub fn new() -> Self {
        Catalog {
            rels: RwLock::new(HashMap::new()),
        }
    }

    /* Rel management */
    pub fn load(&mut self, storage_manager: Arc<StorageManager>) -> Result<()> {
        let mut rels = storage_manager.load_catalog()?;
        let mut write_guard = self.rels.write().unwrap();
        *write_guard = rels.into_iter().map(|rel| (rel.nm.clone(), rel)).collect();
        Ok(())
    }

    pub fn create_table(&self, storage_manager: Arc<StorageManager>, rel: Rel) -> Result<bool> {
        let mut write_guard = self.rels.write().unwrap();
        // first, check if the table already exists
        if write_guard.contains_key(&rel.nm) {
            return Ok(false);
        }
        write_guard.insert(rel.nm.clone(), rel.clone());
        storage_manager.create_rel(rel)?;
        Ok(true)
    }

    pub fn rel_exists_by_name(&self, rel_name: &str) -> bool {
        let read_guard = self.rels.read().unwrap();
        read_guard.contains_key(rel_name)
    }

    pub fn rel_exists_by_id(&self, rel_id: u32) -> bool {
        let read_guard = self.rels.read().unwrap();
        read_guard.values().any(|rel| rel.id == rel_id)
    }

    pub fn delete_rel_by_name(
        &self,
        rel_name: &str,
        storage_manager: Arc<StorageManager>,
    ) -> Result<bool> {
        let mut write_guard = self.rels.write().unwrap();
        if write_guard.remove(rel_name).is_some() {
            storage_manager.delete_rel(rel_name)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn delete_rel_by_id(
        &self,
        rel_id: u32,
        storage_manager: Arc<StorageManager>,
    ) -> Result<bool> {
        let mut write_guard = self.rels.write().unwrap();
        let rel_name_opt = write_guard.iter().find_map(|(name, rel)| {
            if rel.id == rel_id {
                Some(name.clone())
            } else {
                None
            }
        });

        if let Some(rel_name) = rel_name_opt {
            write_guard.remove(&rel_name);
            storage_manager.delete_rel(&rel_name)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
