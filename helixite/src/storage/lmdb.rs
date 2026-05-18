use heed::types::Bytes;
use heed::{Database, Env, RwTxn};
use std::path::Path;

use crate::config::Config;
use crate::error::{HelixiteError, Result};
use crate::storage::engine::{Db, StorageEngine, StorageTxn};

use super::env::open_env;

pub struct LmdbStorage {
    env: Env,
    metadata_db: Database<Bytes, Bytes>,
    nodes_db: Database<Bytes, Bytes>,
    edges_db: Database<Bytes, Bytes>,
    out_edges_db: Database<Bytes, Bytes>,
    in_edges_db: Database<Bytes, Bytes>,
    labels_db: Database<Bytes, Bytes>,
    properties_db: Database<Bytes, Bytes>,
    vector_indexes_db: Database<Bytes, Bytes>,
}

impl LmdbStorage {
    pub fn open(path: &Path, config: &Config) -> Result<Self> {
        let env = open_env(path, config)?;

        let mut wtxn = env.write_txn()?;
        let metadata_db: Database<Bytes, Bytes> =
            env.create_database(&mut wtxn, Some("metadata"))?;
        let nodes_db: Database<Bytes, Bytes> = env.create_database(&mut wtxn, Some("nodes"))?;
        let edges_db: Database<Bytes, Bytes> = env.create_database(&mut wtxn, Some("edges"))?;
        let out_edges_db: Database<Bytes, Bytes> =
            env.create_database(&mut wtxn, Some("out_edges"))?;
        let in_edges_db: Database<Bytes, Bytes> =
            env.create_database(&mut wtxn, Some("in_edges"))?;
        let labels_db: Database<Bytes, Bytes> = env.create_database(&mut wtxn, Some("labels"))?;
        let properties_db: Database<Bytes, Bytes> =
            env.create_database(&mut wtxn, Some("properties"))?;
        let vector_indexes_db: Database<Bytes, Bytes> =
            env.create_database(&mut wtxn, Some("vector_indexes"))?;
        wtxn.commit()?;

        Ok(Self {
            env,
            metadata_db,
            nodes_db,
            edges_db,
            out_edges_db,
            in_edges_db,
            labels_db,
            properties_db,
            vector_indexes_db,
        })
    }

    fn db_for(&self, db: Db) -> Database<Bytes, Bytes> {
        match db {
            Db::Metadata => self.metadata_db,
            Db::Nodes => self.nodes_db,
            Db::Edges => self.edges_db,
            Db::OutEdges => self.out_edges_db,
            Db::InEdges => self.in_edges_db,
            Db::Labels => self.labels_db,
            Db::Properties => self.properties_db,
            Db::VectorIndexes => self.vector_indexes_db,
        }
    }
}

impl StorageEngine for LmdbStorage {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        let database = self.db_for(db);
        Ok(database.get(&rtxn, key)?.map(|b| b.to_vec()))
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let rtxn = self.env.read_txn()?;
        let database = self.db_for(db);
        let iter = database.prefix_iter(&rtxn, prefix)?;
        let mut results = Vec::new();
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
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

    fn db_for(&self, db: Db) -> Database<Bytes, Bytes> {
        self.storage.db_for(db)
    }
}

impl StorageTxn for LmdbTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let database = self.db_for(db);
        Ok(database.get(self.txn()?, key)?.map(|b| b.to_vec()))
    }

    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()> {
        let database = self.db_for(db);
        database.put(self.txn_mut()?, key, value)?;
        Ok(())
    }

    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()> {
        let database = self.db_for(db);
        database.delete(self.txn_mut()?, key)?;
        Ok(())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let database = self.db_for(db);
        let iter = database.prefix_iter(self.txn()?, prefix)?;
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
