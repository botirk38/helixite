use crate::edge::Direction;
use crate::error::Result;
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
    pub fn new(storage: &'a S, node_id: NodeId) -> Self {
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
    pub fn collect_edges(self) -> Result<Vec<crate::edge::Edge>> {
        let (db, prefix) = self.prefix_and_db();
        let entries = self.storage.scan_prefix(db, &prefix)?;
        let mut edges = Vec::new();

        for (key, _) in entries {
            let Some(edge_id) = EdgeIndex::decode_edge_id(&key) else {
                continue;
            };
            let Some(bytes) = self.storage.get(Db::Edges, &edge_id.to_be_bytes())? else {
                continue;
            };
            let edge: crate::edge::Edge = bincode::deserialize(&bytes)
                .map_err(|e| crate::error::HelixiteError::Codec(e.to_string()))?;
            edges.push(edge);
        }

        Ok(edges)
    }

    pub fn collect_nodes(self) -> Result<Vec<Node>> {
        let (db, prefix) = self.prefix_and_db();
        let entries = self.storage.scan_prefix(db, &prefix)?;
        let mut nodes = Vec::new();

        for (key, _) in entries {
            let Some(target_id) = EdgeIndex::decode_target_node(self.storage, &key, self.direction)
            else {
                continue;
            };
            let Some(bytes) = self.storage.get(Db::Nodes, &target_id.to_be_bytes())? else {
                continue;
            };
            let node: Node = bincode::deserialize(&bytes)
                .map_err(|e| crate::error::HelixiteError::Codec(e.to_string()))?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    pub fn count(self) -> Result<usize> {
        let (db, prefix) = self.prefix_and_db();
        let entries = self.storage.scan_prefix(db, &prefix)?;
        Ok(entries.len())
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
