use std::collections::BTreeSet;

use crate::edge::{Direction, Edge};
use crate::error::{IvyError, Result};
use crate::id::NodeId;
use crate::index::edges::EdgeIndex;
use crate::node::Node;
use crate::query::traversal::TraversalQuery;
use crate::storage::ReadTxn;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};

pub struct MultiHopTraversalQuery<'a, S: StorageEngine> {
    storage: &'a S,
    starts: Vec<NodeId>,
    hops: Vec<TraversalHop>,
    limit: Option<usize>,
}

#[derive(Clone)]
struct TraversalHop {
    direction: Direction,
    label: Option<String>,
}

impl TraversalHop {
    fn prefix_and_db(&self, node_id: NodeId) -> (Db, Vec<u8>) {
        match self.direction {
            Direction::Out => (
                Db::OutEdges,
                EdgeIndex::out_prefix(node_id, self.label.as_deref()),
            ),
            Direction::In => (
                Db::InEdges,
                EdgeIndex::in_prefix(node_id, self.label.as_deref()),
            ),
        }
    }
}

impl<'a, S: StorageEngine> MultiHopTraversalQuery<'a, S> {
    pub(super) fn new(storage: &'a S, starts: Vec<NodeId>) -> Self {
        Self {
            storage,
            starts,
            hops: Vec::new(),
            limit: None,
        }
    }

    pub(super) fn from_traversal(query: TraversalQuery<'a, S>) -> Self {
        Self {
            storage: query.storage,
            starts: vec![query.node_id],
            hops: vec![TraversalHop {
                direction: query.direction,
                label: query.label,
            }],
            limit: query.limit,
        }
    }

    pub fn then_outgoing(mut self, label: impl Into<String>) -> Self {
        self.hops.push(TraversalHop {
            direction: Direction::Out,
            label: Some(label.into()),
        });
        self
    }

    pub fn then_incoming(mut self, label: impl Into<String>) -> Self {
        self.hops.push(TraversalHop {
            direction: Direction::In,
            label: Some(label.into()),
        });
        self
    }

    pub fn then_outgoing_any(mut self) -> Self {
        self.hops.push(TraversalHop {
            direction: Direction::Out,
            label: None,
        });
        self
    }

    pub fn then_incoming_any(mut self) -> Self {
        self.hops.push(TraversalHop {
            direction: Direction::In,
            label: None,
        });
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn ids(self) -> Result<Vec<NodeId>> {
        self.storage.read(|txn| {
            let exec = MultiHopTraversalExec {
                txn,
                starts: self.starts,
                hops: self.hops,
                limit: self.limit,
            };
            exec.resolve_node_ids()
        })
    }

    pub fn nodes(self) -> Result<Vec<Node>> {
        self.storage.read(|txn| {
            let exec = MultiHopTraversalExec {
                txn,
                starts: self.starts,
                hops: self.hops,
                limit: self.limit,
            };
            exec.collect_nodes()
        })
    }

    pub fn count(self) -> Result<usize> {
        self.storage.read(|txn| {
            let exec = MultiHopTraversalExec {
                txn,
                starts: self.starts,
                hops: self.hops,
                limit: None,
            };
            exec.resolve_node_ids().map(|ids| ids.len())
        })
    }

    pub fn first_node(self) -> Result<Option<Node>> {
        let mut nodes = self.limit(1).nodes()?;
        Ok(nodes.pop())
    }
}

struct MultiHopTraversalExec<'a> {
    txn: &'a dyn ReadTxn,
    starts: Vec<NodeId>,
    hops: Vec<TraversalHop>,
    limit: Option<usize>,
}

impl MultiHopTraversalExec<'_> {
    fn collect_nodes(self) -> Result<Vec<Node>> {
        let ids = self.resolve_node_ids()?;
        ids.into_iter().map(|id| self.load_node(id)).collect()
    }

    fn resolve_node_ids(&self) -> Result<Vec<NodeId>> {
        let mut current: BTreeSet<NodeId> = self.starts.iter().copied().collect();
        if self.hops.is_empty() {
            return Ok(self.limited_ids(current));
        }

        for hop in &self.hops {
            let mut next = BTreeSet::new();
            for node_id in &current {
                let (db, prefix) = hop.prefix_and_db(*node_id);
                for entry in self.txn.iter(db, Scan::Prefix(&prefix))? {
                    let entry = entry?;
                    let edge = self.load_edge_from_key(entry.key)?;
                    next.insert(EdgeIndex::decode_target_from_edge(&edge, hop.direction));
                }
            }
            current = next;
            if current.is_empty() {
                break;
            }
        }

        Ok(self.limited_ids(current))
    }

    fn limited_ids(&self, ids: BTreeSet<NodeId>) -> Vec<NodeId> {
        let mut ids: Vec<_> = ids.into_iter().collect();
        if let Some(limit) = self.limit {
            ids.truncate(limit);
        }
        ids
    }

    fn load_edge_from_key(&self, key: &[u8]) -> Result<Edge> {
        let edge_id = EdgeIndex::decode_edge_id(key)
            .ok_or_else(|| IvyError::Storage("corrupt edge adjacency key".into()))?;
        let bytes = self
            .txn
            .get(Db::Edges, &edge_id.to_be_bytes())?
            .ok_or(IvyError::EdgeNotFound(edge_id))?;
        bincode::deserialize(&bytes).map_err(|e| IvyError::Codec(e.to_string()))
    }

    fn load_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .txn
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(IvyError::NodeNotFound(id))?;
        bincode::deserialize(&bytes).map_err(|e| IvyError::Codec(e.to_string()))
    }
}
