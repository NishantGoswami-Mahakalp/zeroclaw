use crate::config::ConfigDatabase;
use std::path::PathBuf;

pub struct ConfigState {
    pub db: Option<ConfigDatabase>,
    pub data_dir: PathBuf,
}

impl ConfigState {
    pub fn new(data_dir: PathBuf) -> Self {
        let db = ConfigDatabase::new(&data_dir).ok();
        Self { db, data_dir }
    }

    pub fn is_enabled(&self) -> bool {
        self.db.is_some()
    }

    pub fn db(&self) -> Option<&ConfigDatabase> {
        self.db.as_ref()
    }
}
