use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::storage::engine::{Db, Entry, EntryIter, ReadTxn, Scan, StorageEngine, WriteTxn};

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
    fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn ReadTxn) -> Result<T>,
    {
        let snapshot = self.data.lock().unwrap().clone();
        let txn = MemoryReadTxn { snapshot };
        f(&txn)
    }

    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn WriteTxn) -> Result<T>,
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
}

impl ReadTxn for MemoryTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.snapshot.get(&(db, key.to_vec())).cloned())
    }

    fn iter<'a>(&'a self, db: Db, scan: Scan<'a>) -> Result<EntryIter<'a>> {
        Ok(Box::new(
            self.snapshot
                .iter()
                .filter(move |((stored_db, key), _)| {
                    *stored_db == db
                        && match scan {
                            Scan::All => true,
                            Scan::Prefix(prefix) => key.starts_with(prefix),
                        }
                })
                .map(|((_, key), value)| {
                    Ok(Entry {
                        key: key.as_slice(),
                        value: value.as_slice(),
                    })
                }),
        ))
    }
}

impl WriteTxn for MemoryTxn<'_> {
    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()> {
        self.snapshot.insert((db, key.to_vec()), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()> {
        self.snapshot.remove(&(db, key.to_vec()));
        Ok(())
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

impl ReadTxn for MemoryReadTxn {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.snapshot.get(&(db, key.to_vec())).cloned())
    }

    fn iter<'a>(&'a self, db: Db, scan: Scan<'a>) -> Result<EntryIter<'a>> {
        Ok(Box::new(
            self.snapshot
                .iter()
                .filter(move |((stored_db, key), _)| {
                    *stored_db == db
                        && match scan {
                            Scan::All => true,
                            Scan::Prefix(prefix) => key.starts_with(prefix),
                        }
                })
                .map(|((_, key), value)| {
                    Ok(Entry {
                        key: key.as_slice(),
                        value: value.as_slice(),
                    })
                }),
        ))
    }
}
