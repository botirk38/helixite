use helixite::storage::{Db, StorageEngine, StorageTxn};
use helixite::{Config, HelixiteBuilder};
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::tempdir;

type MemoryKey = (Db, Vec<u8>);

#[derive(Default)]
struct MemoryStorage {
    data: Mutex<HashMap<MemoryKey, Vec<u8>>>,
}

struct MemoryTxn<'a> {
    data: &'a Mutex<HashMap<MemoryKey, Vec<u8>>>,
}

impl StorageEngine for MemoryStorage {
    fn get(&self, db: Db, key: &[u8]) -> helixite::Result<Option<Vec<u8>>> {
        Ok(self.data.lock().unwrap().get(&(db, key.to_vec())).cloned())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> helixite::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }

    fn write<F, T>(&self, f: F) -> helixite::Result<T>
    where
        F: FnOnce(&mut dyn StorageTxn) -> helixite::Result<T>,
    {
        f(&mut MemoryTxn { data: &self.data })
    }
}

impl StorageTxn for MemoryTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> helixite::Result<Option<Vec<u8>>> {
        Ok(self.data.lock().unwrap().get(&(db, key.to_vec())).cloned())
    }

    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> helixite::Result<()> {
        self.data
            .lock()
            .unwrap()
            .insert((db, key.to_vec()), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, db: Db, key: &[u8]) -> helixite::Result<()> {
        self.data.lock().unwrap().remove(&(db, key.to_vec()));
        Ok(())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> helixite::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }
}

#[test]
fn test_open_new_db() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();
    assert!(db.path().exists());
}

#[test]
fn test_reopen_db() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let db1 = HelixiteBuilder::default().open(path).unwrap();
    drop(db1);

    let db2 = HelixiteBuilder::default().open(path).unwrap();
    assert!(db2.path().exists());
}

#[test]
fn test_open_with_config() {
    let dir = tempdir().unwrap();
    let config = Config::default().with_map_size(64 * 1024 * 1024);
    let db = HelixiteBuilder::default().config(config).open(dir.path()).unwrap();
    assert!(db.path().exists());
}

#[test]
fn test_open_with_storage_accepts_custom_engine() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default()
        .storage(MemoryStorage::default())
        .open(dir.path())
        .unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();
    let node = db.get_node(id).unwrap();

    assert_eq!(node.label, "User");
}
