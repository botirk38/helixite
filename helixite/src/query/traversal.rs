use crate::edge::{Direction, Edge};
use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;

use crate::index::edges::EdgeIndex;

pub struct NodeRefQuery<'a, S: StorageEngine> {
    storage: &'a S,
    node_id: NodeId,
}

pub struct TraversalQuery<'a, S: StorageEngine> {
    storage: &'a S,
    node_id: NodeId,
    direction: Direction,
    label: Option<String>,
}

impl<'a, S: StorageEngine> NodeRefQuery<'a, S> {
    pub(crate) fn new(storage: &'a S, node_id: NodeId) -> Self {
        Self { storage, node_id }
    }

    pub fn out(self, label: impl Into<String>) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::Out,
            label: Some(label.into()),
        }
    }

    pub fn in_(self, label: impl Into<String>) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::In,
            label: Some(label.into()),
        }
    }

    pub fn out_any(self) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::Out,
            label: None,
        }
    }

    pub fn in_any(self) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::In,
            label: None,
        }
    }
}

impl<'a, S: StorageEngine> TraversalQuery<'a, S> {
    pub fn collect_edges(self) -> Result<Vec<Edge>> {
        let (db, prefix) = self.prefix_and_db();
        let entries = self.storage.scan_prefix(db, &prefix)?;
        let mut edges = Vec::with_capacity(entries.len());

        for (key, _) in entries {
            let edge_id = EdgeIndex::decode_edge_id(&key)
                .ok_or_else(|| HelixiteError::Storage("corrupt edge adjacency key".into()))?;
            let edge = self.load_edge(edge_id)?;
            edges.push(edge);
        }

        Ok(edges)
    }

    pub fn collect_nodes(self) -> Result<Vec<Node>> {
        let (db, prefix) = self.prefix_and_db();
        let entries = self.storage.scan_prefix(db, &prefix)?;
        let mut nodes = Vec::with_capacity(entries.len());

        for (key, _) in entries {
            let edge = self.load_edge_from_key(&key)?;
            let target_id = EdgeIndex::decode_target_from_edge(&edge, self.direction);
            let node = self.load_node(target_id)?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    pub fn count(self) -> Result<usize> {
        let (db, prefix) = self.prefix_and_db();
        let entries = self.storage.scan_prefix(db, &prefix)?;
        Ok(entries.len())
    }

    fn load_edge(&self, edge_id: NodeId) -> Result<Edge> {
        let bytes = self
            .storage
            .get(Db::Edges, &edge_id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(edge_id))?;
        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    fn load_edge_from_key(&self, key: &[u8]) -> Result<Edge> {
        let edge_id = EdgeIndex::decode_edge_id(key)
            .ok_or_else(|| HelixiteError::Storage("corrupt edge adjacency key".into()))?;
        self.load_edge(edge_id)
    }

    fn load_node(&self, node_id: NodeId) -> Result<Node> {
        let bytes = self
            .storage
            .get(Db::Nodes, &node_id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(node_id))?;
        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    fn prefix_and_db(&self) -> (Db, Vec<u8>) {
        match self.direction {
            Direction::Out => (
                Db::OutEdges,
                EdgeIndex::out_prefix(self.node_id, self.label.as_deref()),
            ),
            Direction::In => (
                Db::InEdges,
                EdgeIndex::in_prefix(self.node_id, self.label.as_deref()),
            ),
        }
    }
}
