use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::edge::Edge;
use crate::error::Result;
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::query::{NodeQuery, NodeRefQuery};
use crate::storage::StorageEngine;
use crate::storage::lmdb::LmdbStorage;
use crate::value::Value;

use crate::command::IndexManager;
use crate::txn::{EdgeMutBuilder, NodeMutBuilder, ReadTxn, WriteTxn};

pub struct Helixite<S: StorageEngine = LmdbStorage> {
    path: PathBuf,
    storage: S,
}

pub struct HelixiteBuilder {
    config: Config,
}

impl HelixiteBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }
}

impl Default for HelixiteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HelixiteStorageBuilder<S: StorageEngine> {
    storage: S,
}

impl HelixiteBuilder {
    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn storage<S: StorageEngine>(self, storage: S) -> HelixiteStorageBuilder<S> {
        HelixiteStorageBuilder { storage }
    }

    pub fn open(self, path: impl AsRef<Path>) -> Result<Helixite> {
        let path = path.as_ref().to_path_buf();
        let storage = LmdbStorage::open(&path, &self.config)?;
        Ok(Helixite { path, storage })
    }
}

impl<S: StorageEngine> HelixiteStorageBuilder<S> {
    pub fn open(self, path: impl AsRef<Path>) -> Result<Helixite<S>> {
        Ok(Helixite {
            path: path.as_ref().to_path_buf(),
            storage: self.storage,
        })
    }
}

impl<S: StorageEngine> Helixite<S> {
    pub fn add_node(
        &self,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<NodeId> {
        self.write(|tx| tx.add_node(label, properties))
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        self.read(|tx| tx.get_node(id))
    }

    pub fn add_edge(
        &self,
        from: NodeId,
        to: NodeId,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<EdgeId> {
        self.write(|tx| tx.add_edge(from, to, label, properties))
    }

    pub fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        self.read(|tx| tx.get_edge(id))
    }

    pub fn delete_edge(&self, id: EdgeId) -> Result<()> {
        self.write(|tx| tx.delete_edge(id))
    }

    pub fn delete_node(&self, id: NodeId) -> Result<()> {
        self.write(|tx| tx.delete_node(id))
    }

    pub fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut WriteTxn<'_>) -> Result<T>,
    {
        self.storage.write(|txn| {
            let mut tx = WriteTxn::new(txn);
            f(&mut tx)
        })
    }

    pub fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&ReadTxn<'_>) -> Result<T>,
    {
        self.storage.read(|txn| {
            let tx = ReadTxn::new(txn);
            f(&tx)
        })
    }

    pub fn get_nodes(&self, ids: &[NodeId]) -> Result<Vec<Node>> {
        self.read(|tx| tx.get_nodes(ids))
    }

    pub fn get_edges(&self, ids: &[EdgeId]) -> Result<Vec<Edge>> {
        self.read(|tx| tx.get_edges(ids))
    }

    pub fn update_node(&self, id: NodeId) -> NodeMutBuilder<'_, S> {
        NodeMutBuilder::new(self, id)
    }

    pub fn update_edge(&self, id: EdgeId) -> EdgeMutBuilder<'_, S> {
        EdgeMutBuilder::new(self, id)
    }

    pub fn nodes(&self) -> NodeQuery<'_, S> {
        NodeQuery::new(&self.storage)
    }

    pub fn node(&self, id: NodeId) -> NodeRefQuery<'_, S> {
        NodeRefQuery::new(&self.storage, id)
    }

    pub fn indexes(&self) -> IndexManager<'_, S> {
        IndexManager::new(&self.storage)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }
}
