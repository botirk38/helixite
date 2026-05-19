use std::path::{Path, PathBuf};

use crate::command::{EdgeCommand, IndexManager, NodeCommand};
use crate::config::Config;
use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::query::{NodeQuery, NodeRefQuery};
use crate::storage::engine::Db;
use crate::storage::lmdb::LmdbStorage;
use crate::storage::{IdAllocator, StorageEngine};
use crate::value::Value;

use crate::index::edges::EdgeIndex;
use crate::index::nodes::NodeIndexes;
use crate::index::properties::EdgePropertyIndexes;
use crate::index::properties::PropertyIndexRegistry;

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
        let label = label.into();
        let properties: std::collections::BTreeMap<String, Value> =
            properties.into_iter().collect();

        NodeIndexes::validate(&self.storage, &label, &properties)?;

        let node_id = self.storage.write(|txn| {
            let registered = PropertyIndexRegistry::load_nodes_from_txn(txn)?;

            let next_id = IdAllocator::next_node_id(txn)?;

            let node = Node {
                id: next_id,
                label: label.clone(),
                properties: properties.clone(),
            };

            let bytes =
                bincode::serialize(&node).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Nodes, &next_id.to_be_bytes(), &bytes)?;

            NodeIndexes::insert(txn, &label, next_id, &properties, &registered)?;

            Ok(next_id)
        })?;

        Ok(node_id)
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .storage
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        let node = bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(node)
    }

    pub fn node_mut(&self, id: NodeId) -> NodeCommand<'_, S> {
        NodeCommand::new(self, id)
    }

    pub fn add_edge(
        &self,
        from: NodeId,
        to: NodeId,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<EdgeId> {
        let label = label.into();
        let properties: std::collections::BTreeMap<String, Value> =
            properties.into_iter().collect();

        let edge_id = self.storage.write(|txn| {
            let registered = PropertyIndexRegistry::load_edges_from_txn(txn)?;

            let next_id = IdAllocator::next_edge_id(txn)?;

            let edge = Edge {
                id: next_id,
                from,
                to,
                label: label.clone(),
                properties,
            };

            let bytes =
                bincode::serialize(&edge).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Edges, &next_id.to_be_bytes(), &bytes)?;

            EdgeIndex::insert(txn, from, to, &label, next_id)?;
            EdgePropertyIndexes::insert(txn, &registered, &edge)?;

            Ok(next_id)
        })?;

        Ok(edge_id)
    }

    pub fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        let bytes = self
            .storage
            .get(Db::Edges, &id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(id))?;

        let edge = bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(edge)
    }

    pub fn edge_mut(&self, id: EdgeId) -> EdgeCommand<'_, S> {
        EdgeCommand::new(self, id)
    }

    pub fn delete_edge(&self, id: EdgeId) -> Result<()> {
        let edge = self.get_edge(id)?;

        self.storage.write(|txn| {
            let registered = PropertyIndexRegistry::load_edges_from_txn(txn)?;

            EdgeIndex::delete(txn, edge.from, edge.to, &edge.label, edge.id)?;
            EdgePropertyIndexes::delete(txn, &registered, &edge)?;
            txn.delete(Db::Edges, &edge.id.to_be_bytes())?;

            Ok(())
        })
    }

    pub fn delete_node(&self, id: NodeId) -> Result<()> {
        let node = self.get_node(id)?;

        self.storage.write(|txn| {
            let node_registry = PropertyIndexRegistry::load_nodes_from_txn(txn)?;
            let edge_registry = PropertyIndexRegistry::load_edges_from_txn(txn)?;

            let out_prefix = EdgeIndex::out_prefix(id, None);
            let out_entries = txn.scan_prefix(Db::OutEdges, &out_prefix)?;

            for (key, _) in &out_entries {
                let Some(edge_id) = EdgeIndex::decode_edge_id(key) else {
                    continue;
                };
                self.delete_edge_from_txn(txn, &edge_registry, edge_id)?;
            }

            let in_prefix = EdgeIndex::in_prefix(id, None);
            let in_entries = txn.scan_prefix(Db::InEdges, &in_prefix)?;

            for (key, _) in &in_entries {
                let Some(edge_id) = EdgeIndex::decode_edge_id(key) else {
                    continue;
                };
                self.delete_edge_from_txn(txn, &edge_registry, edge_id)?;
            }

            NodeIndexes::delete(txn, &node, &node_registry)?;
            txn.delete(Db::Nodes, &node.id.to_be_bytes())?;

            Ok(())
        })
    }

    fn delete_edge_from_txn(
        &self,
        txn: &mut dyn crate::storage::StorageTxn,
        edge_registry: &PropertyIndexRegistry,
        edge_id: EdgeId,
    ) -> Result<()> {
        let bytes = match txn.get(Db::Edges, &edge_id.to_be_bytes())? {
            Some(b) => b,
            None => return Ok(()),
        };

        let edge: Edge =
            bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        EdgeIndex::delete(txn, edge.from, edge.to, &edge.label, edge.id)?;
        EdgePropertyIndexes::delete(txn, edge_registry, &edge)?;
        txn.delete(Db::Edges, &edge.id.to_be_bytes())?;

        Ok(())
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
