use crate::schema::{self, Schema};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug)]
pub struct Fmanager {
    pub schs_dir_buf: PathBuf,
    pub fschs: Vec<FileSchema>,
}

#[derive(Debug)]
pub struct FileSchema {
    pub schema: Option<Arc<Schema>>,
    pub sch_path_buf: PathBuf,
}

impl Fmanager {
    pub fn new<P: Into<PathBuf>>(dir_path: P) -> Self {
        Fmanager {
            schs_dir_buf: dir_path.into(),
            fschs: vec![],
        }
    }

    // pub fn load(&self) -> bool {

    // }
}

impl FileSchema {
    pub fn new<P: Into<PathBuf>>(sch_path: P) -> Self {
        FileSchema {
            schema: None,
            sch_path_buf: sch_path.into(),
        }
    }
}
