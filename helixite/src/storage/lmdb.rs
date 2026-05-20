use heed::types::Bytes;
use heed::{Database, Env, RoTxn, RwTxn};
use std::path::Path;

use crate::config::Config;
use crate::error::{HelixiteError, Result};
use crate::storage::engine::{Db, Entry, EntryIter, ReadTxn, Scan, StorageEngine, WriteTxn};

use super::env::open_env;

pub struct LmdbStorage {
    env: Env,
    dbs: [Database<Bytes, Bytes>; Db::COUNT],
}

impl LmdbStorage {
    pub fn open(path: &Path, config: &Config) -> Result<Self> {
        let env = open_env(path, config)?;

        let mut wtxn = env.write_txn()?;
        let mut dbs = [None; Db::COUNT];

        for db in Db::ALL {
            dbs[db.index()] = Some(env.create_database(&mut wtxn, Some(db.name()))?);
        }

        wtxn.commit()?;

        let dbs = dbs.map(|db| db.expect("all dbs initialized"));

        Ok(Self { env, dbs })
    }

    fn database(&self, db: Db) -> Database<Bytes, Bytes> {
        self.dbs[db.index()]
    }
}

impl StorageEngine for LmdbStorage {
    fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn ReadTxn) -> Result<T>,
    {
        let lmdb_txn = LmdbReadTxn::new(self)?;
        f(&lmdb_txn)
    }

    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn WriteTxn) -> Result<T>,
    {
        let mut lmdb_txn = LmdbTxn::new(self)?;
        let result = f(&mut lmdb_txn);
        if result.is_ok() {
            lmdb_txn.commit()?;
        }
        result
    }
}

struct LmdbTxn<'e> {
    storage: &'e LmdbStorage,
    txn: Option<RwTxn<'e>>,
}

impl<'e> LmdbTxn<'e> {
    fn new(storage: &'e LmdbStorage) -> Result<Self> {
        Ok(Self {
            storage,
            txn: Some(storage.env.write_txn()?),
        })
    }

    fn commit(&mut self) -> Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.commit()?;
        }
        Ok(())
    }

    fn txn(&self) -> Result<&RwTxn<'e>> {
        self.txn
            .as_ref()
            .ok_or_else(|| HelixiteError::Storage("transaction already closed".into()))
    }

    fn txn_mut(&mut self) -> Result<&mut RwTxn<'e>> {
        self.txn
            .as_mut()
            .ok_or_else(|| HelixiteError::Storage("transaction already closed".into()))
    }

    fn database(&self, db: Db) -> Database<Bytes, Bytes> {
        self.storage.database(db)
    }
}

impl ReadTxn for LmdbTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let database = self.database(db);
        Ok(database.get(self.txn()?, key)?.map(|b| b.to_vec()))
    }

    fn iter<'a>(&'a self, db: Db, scan: Scan<'a>) -> Result<EntryIter<'a>> {
        let database = self.database(db);
        match scan {
            Scan::All => {
                let iter = database.iter(self.txn()?)?;
                Ok(Box::new(iter.map(|r| {
                    let (k, v) = r?;
                    Ok(Entry { key: k, value: v })
                })))
            }
            Scan::Prefix(prefix) => {
                let iter = database.prefix_iter(self.txn()?, prefix)?;
                Ok(Box::new(iter.map(|r| {
                    let (k, v) = r?;
                    Ok(Entry { key: k, value: v })
                })))
            }
        }
    }
}

impl WriteTxn for LmdbTxn<'_> {
    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()> {
        let database = self.database(db);
        database.put(self.txn_mut()?, key, value)?;
        Ok(())
    }

    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()> {
        let database = self.database(db);
        database.delete(self.txn_mut()?, key)?;
        Ok(())
    }
}

impl Drop for LmdbTxn<'_> {
    fn drop(&mut self) {
        if let Some(txn) = self.txn.take() {
            txn.abort();
        }
    }
}

struct LmdbReadTxn<'e> {
    storage: &'e LmdbStorage,
    txn: RoTxn<'e>,
}

impl<'e> LmdbReadTxn<'e> {
    fn new(storage: &'e LmdbStorage) -> Result<Self> {
        Ok(Self {
            storage,
            txn: storage.env.read_txn()?,
        })
    }

    fn txn(&self) -> &RoTxn<'e> {
        &self.txn
    }

    fn database(&self, db: Db) -> Database<Bytes, Bytes> {
        self.storage.database(db)
    }
}

impl ReadTxn for LmdbReadTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let database = self.database(db);
        Ok(database.get(self.txn(), key)?.map(|b| b.to_vec()))
    }

    fn iter<'a>(&'a self, db: Db, scan: Scan<'a>) -> Result<EntryIter<'a>> {
        let database = self.database(db);
        match scan {
            Scan::All => {
                let iter = database.iter(self.txn())?;
                Ok(Box::new(iter.map(|r| {
                    let (k, v) = r?;
                    Ok(Entry { key: k, value: v })
                })))
            }
            Scan::Prefix(prefix) => {
                let iter = database.prefix_iter(self.txn(), prefix)?;
                Ok(Box::new(iter.map(|r| {
                    let (k, v) = r?;
                    Ok(Entry { key: k, value: v })
                })))
            }
        }
    }
}
