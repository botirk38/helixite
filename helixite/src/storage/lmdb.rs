use heed::types::Bytes;
use heed::{Database, Env, RoTxn, RwTxn};
use std::path::Path;

use crate::config::Config;
use crate::error::{HelixiteError, Result};
use crate::storage::engine::{Db, StorageEngine, StorageTxn};

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
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        let database = self.database(db);
        Ok(database.get(&rtxn, key)?.map(|b| b.to_vec()))
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let rtxn = self.env.read_txn()?;
        let database = self.database(db);
        let iter = database.prefix_iter(&rtxn, prefix)?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
    }

    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let rtxn = self.env.read_txn()?;
        let database = self.database(db);
        let iter = database.iter(&rtxn)?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
    }

    fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn StorageTxn) -> Result<T>,
    {
        let lmdb_txn = LmdbReadTxn::new(self)?;
        f(&lmdb_txn)
    }

    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn StorageTxn) -> Result<T>,
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

impl StorageTxn for LmdbTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let database = self.database(db);
        Ok(database.get(self.txn()?, key)?.map(|b| b.to_vec()))
    }

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

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let database = self.database(db);
        let iter = database.prefix_iter(self.txn()?, prefix)?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
    }

    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let database = self.database(db);
        let iter = database.iter(self.txn()?)?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
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

impl StorageTxn for LmdbReadTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let database = self.database(db);
        Ok(database.get(self.txn(), key)?.map(|b| b.to_vec()))
    }

    fn put(&mut self, _db: Db, _key: &[u8], _value: &[u8]) -> Result<()> {
        Err(HelixiteError::Storage(
            "read-only transaction does not support writes".into(),
        ))
    }

    fn delete(&mut self, _db: Db, _key: &[u8]) -> Result<()> {
        Err(HelixiteError::Storage(
            "read-only transaction does not support deletes".into(),
        ))
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let database = self.database(db);
        let iter = database.prefix_iter(self.txn(), prefix)?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
    }

    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let database = self.database(db);
        let iter = database.iter(self.txn())?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
    }
}
