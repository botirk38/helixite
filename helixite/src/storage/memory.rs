use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::storage::engine::{Db, StorageEngine, StorageTxn};

type MemoryKey = (Db, Vec<u8>);

pub struct MemoryStorage {
    data: Arc<Mutex<HashMap<MemoryKey, Vec<u8>>>>,
}

struct MemoryTxn<'a> {
    data: &'a Mutex<HashMap<MemoryKey, Vec<u8>>>,
    snapshot: HashMap<MemoryKey, Vec<u8>>,
    committed: bool,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl StorageEngine for MemoryStorage {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.data.lock().unwrap().get(&(db, key.to_vec())).cloned())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }

    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .iter()
            .filter(|((stored_db, _), _)| *stored_db == db)
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }

    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn StorageTxn) -> Result<T>,
    {
        let snapshot = self.data.lock().unwrap().clone();
        let mut txn = MemoryTxn {
            data: &self.data,
            snapshot,
            committed: false,
        };
        let result = f(&mut txn);
        if result.is_ok() {
            txn.committed = true;
        }
        result
    }

    fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn StorageTxn) -> Result<T>,
    {
        let snapshot = self.data.lock().unwrap().clone();
        let txn = MemoryReadTxn { snapshot };
        f(&txn)
    }
}

impl StorageTxn for MemoryTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.snapshot.get(&(db, key.to_vec())).cloned())
    }

    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()> {
        self.snapshot.insert((db, key.to_vec()), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()> {
        self.snapshot.remove(&(db, key.to_vec()));
        Ok(())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .snapshot
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }

    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .snapshot
            .iter()
            .filter(|((stored_db, _), _)| *stored_db == db)
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }
}

impl Drop for MemoryTxn<'_> {
    fn drop(&mut self) {
        if self.committed {
            let mut data = self.data.lock().unwrap();
            for (key, value) in self.snapshot.drain() {
                data.insert(key, value);
            }
        }
    }
}

struct MemoryReadTxn {
    snapshot: HashMap<MemoryKey, Vec<u8>>,
}

impl StorageTxn for MemoryReadTxn {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.snapshot.get(&(db, key.to_vec())).cloned())
    }

    fn put(&mut self, _db: Db, _key: &[u8], _value: &[u8]) -> Result<()> {
        Err(crate::error::HelixiteError::Storage(
            "cannot write in a read-only transaction".into(),
        ))
    }

    fn delete(&mut self, _db: Db, _key: &[u8]) -> Result<()> {
        Err(crate::error::HelixiteError::Storage(
            "cannot write in a read-only transaction".into(),
        ))
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .snapshot
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }

    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .snapshot
            .iter()
            .filter(|((stored_db, _), _)| *stored_db == db)
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }
}
