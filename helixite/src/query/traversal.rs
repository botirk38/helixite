use std::collections::BTreeSet;

use crate::edge::{Direction, Edge};
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::storage::ReadTxn;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};
use crate::value::Value;

use crate::index::edges::EdgeIndex;
use crate::index::properties::EdgePropertyIndex;
use crate::index::properties::PropertyIndexMetadata;

#[derive(Debug, Clone)]
pub(crate) enum EdgePropertyFilter {
    Eq(String, Value),
}

pub struct NodeRefQuery<'a, S: StorageEngine> {
    storage: &'a S,
    node_id: NodeId,
}

pub struct TraversalQuery<'a, S: StorageEngine> {
    storage: &'a S,
    node_id: NodeId,
    direction: Direction,
    label: Option<String>,
    filters: Vec<EdgePropertyFilter>,
    limit: Option<usize>,
}

impl<'a, S: StorageEngine> NodeRefQuery<'a, S> {
    pub(crate) fn new(storage: &'a S, node_id: NodeId) -> Self {
        Self { storage, node_id }
    }

    pub fn outgoing(self, label: impl Into<String>) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::Out,
            label: Some(label.into()),
            filters: Vec::new(),
            limit: None,
        }
    }

    pub fn incoming(self, label: impl Into<String>) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::In,
            label: Some(label.into()),
            filters: Vec::new(),
            limit: None,
        }
    }

    pub fn outgoing_any(self) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::Out,
            label: None,
            filters: Vec::new(),
            limit: None,
        }
    }

    pub fn incoming_any(self) -> TraversalQuery<'a, S> {
        TraversalQuery {
            storage: self.storage,
            node_id: self.node_id,
            direction: Direction::In,
            label: None,
            filters: Vec::new(),
            limit: None,
        }
    }
}

impl<'a, S: StorageEngine> TraversalQuery<'a, S> {
    pub fn eq(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(EdgePropertyFilter::Eq(property.into(), value));
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn first_edge(self) -> Result<Option<Edge>> {
        let edges = self.limit(1).edges()?;
        Ok(edges.into_iter().next())
    }

    pub fn first_node(self) -> Result<Option<Node>> {
        let nodes = self.limit(1).nodes()?;
        Ok(nodes.into_iter().next())
    }

    pub fn edges(self) -> Result<Vec<Edge>> {
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: self.limit,
            };
            exec.collect_edges()
        })
    }

    pub fn nodes(self) -> Result<Vec<Node>> {
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: self.limit,
            };
            exec.collect_nodes()
        })
    }

    pub fn count(self) -> Result<usize> {
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: None,
            };
            exec.count()
        })
    }
}

struct TraversalExec<'a> {
    txn: &'a dyn ReadTxn,
    node_id: NodeId,
    direction: Direction,
    label: Option<String>,
    filters: Vec<EdgePropertyFilter>,
    limit: Option<usize>,
}

impl TraversalExec<'_> {
    fn collect_edges(self) -> Result<Vec<Edge>> {
        if !self.filters.is_empty() {
            return self.collect_edges_filtered();
        }

        let (db, prefix) = self.prefix_and_db();
        let mut edges = Vec::new();

        for entry in self.txn.iter(db, Scan::Prefix(&prefix))? {
            let entry = entry?;
            let edge_id = EdgeIndex::decode_edge_id(entry.key)
                .ok_or_else(|| HelixiteError::Storage("corrupt edge adjacency key".into()))?;
            let edge = self.load_edge(edge_id)?;
            edges.push(edge);
            if let Some(limit) = self.limit
                && edges.len() >= limit
            {
                break;
            }
        }

        Ok(edges)
    }

    fn collect_nodes(self) -> Result<Vec<Node>> {
        if !self.filters.is_empty() {
            return self.collect_nodes_filtered();
        }

        let (db, prefix) = self.prefix_and_db();
        let mut nodes = Vec::new();

        for entry in self.txn.iter(db, Scan::Prefix(&prefix))? {
            let entry = entry?;
            let edge = self.load_edge_from_key(entry.key)?;
            let target_id = EdgeIndex::decode_target_from_edge(&edge, self.direction);
            let node = self.load_node(target_id)?;
            nodes.push(node);
            if let Some(limit) = self.limit
                && nodes.len() >= limit
            {
                break;
            }
        }

        Ok(nodes)
    }

    fn count(self) -> Result<usize> {
        if !self.filters.is_empty() {
            return self.count_filtered();
        }

        let (db, prefix) = self.prefix_and_db();
        let mut count = 0;
        for entry in self.txn.iter(db, Scan::Prefix(&prefix))? {
            entry?;
            count += 1;
        }
        Ok(count)
    }

    fn collect_edges_filtered(self) -> Result<Vec<Edge>> {
        let edge_label = self.label.as_ref().ok_or_else(|| {
            HelixiteError::IndexNotFound("edge property filter requires a label".to_string())
        })?;

        for filter in &self.filters {
            let EdgePropertyFilter::Eq(property, _value) = filter;
            if !self.is_edge_property_indexed(edge_label, property)? {
                return Err(HelixiteError::IndexNotFound(format!(
                    "no edge property index for {edge_label}::{property}"
                )));
            }
        }

        let candidate_ids = self.resolve_filtered_edge_ids(edge_label)?;
        let adjacency_ids = self.scan_adjacency_ids()?;

        let mut edges = Vec::new();
        for edge_id in &candidate_ids {
            if adjacency_ids.contains(edge_id) {
                let edge = self.load_edge(*edge_id)?;
                edges.push(edge);
                if let Some(limit) = self.limit
                    && edges.len() >= limit
                {
                    break;
                }
            }
        }

        Ok(edges)
    }

    fn collect_nodes_filtered(self) -> Result<Vec<Node>> {
        let edge_label = self.label.as_ref().ok_or_else(|| {
            HelixiteError::IndexNotFound("edge property filter requires a label".to_string())
        })?;

        for filter in &self.filters {
            let EdgePropertyFilter::Eq(property, _value) = filter;
            if !self.is_edge_property_indexed(edge_label, property)? {
                return Err(HelixiteError::IndexNotFound(format!(
                    "no edge property index for {edge_label}::{property}"
                )));
            }
        }

        let candidate_ids = self.resolve_filtered_edge_ids(edge_label)?;
        let adjacency_ids = self.scan_adjacency_ids()?;

        let mut nodes = Vec::new();
        for edge_id in &candidate_ids {
            if adjacency_ids.contains(edge_id) {
                let edge = self.load_edge(*edge_id)?;
                let target_id = EdgeIndex::decode_target_from_edge(&edge, self.direction);
                let node = self.load_node(target_id)?;
                nodes.push(node);
                if let Some(limit) = self.limit
                    && nodes.len() >= limit
                {
                    break;
                }
            }
        }

        Ok(nodes)
    }

    fn count_filtered(self) -> Result<usize> {
        let edge_label = self.label.as_ref().ok_or_else(|| {
            HelixiteError::IndexNotFound("edge property filter requires a label".to_string())
        })?;

        for filter in &self.filters {
            let EdgePropertyFilter::Eq(property, _value) = filter;
            if !self.is_edge_property_indexed(edge_label, property)? {
                return Err(HelixiteError::IndexNotFound(format!(
                    "no edge property index for {edge_label}::{property}"
                )));
            }
        }

        let candidate_ids = self.resolve_filtered_edge_ids(edge_label)?;
        let adjacency_ids = self.scan_adjacency_ids()?;

        Ok(candidate_ids
            .iter()
            .filter(|id| adjacency_ids.contains(id))
            .count())
    }

    fn resolve_filtered_edge_ids(&self, label: &str) -> Result<BTreeSet<EdgeId>> {
        let mut sets: Vec<BTreeSet<EdgeId>> = Vec::with_capacity(self.filters.len());

        for filter in &self.filters {
            let EdgePropertyFilter::Eq(property, value) = filter;
            let Some(prefix) = EdgePropertyIndex::lookup_prefix(label, property, value) else {
                return Ok(BTreeSet::new());
            };

            let mut set = BTreeSet::new();
            for entry in self.txn.iter(Db::Properties, Scan::Prefix(&prefix))? {
                let entry = entry?;
                if let Some(edge_id) = EdgePropertyIndex::decode_edge_id(entry.key) {
                    set.insert(edge_id);
                }
            }
            sets.push(set);
        }

        if sets.is_empty() {
            return Ok(BTreeSet::new());
        }

        let mut result = sets[0].clone();
        for set in &sets[1..] {
            result = result.intersection(set).copied().collect();
        }

        Ok(result)
    }

    fn scan_adjacency_ids(&self) -> Result<BTreeSet<EdgeId>> {
        let (db, prefix) = self.prefix_and_db();
        let mut ids = BTreeSet::new();
        for entry in self.txn.iter(db, Scan::Prefix(&prefix))? {
            let entry = entry?;
            if let Some(edge_id) = EdgeIndex::decode_edge_id(entry.key) {
                ids.insert(edge_id);
            }
        }
        Ok(ids)
    }

    fn is_edge_property_indexed(&self, label: &str, property: &str) -> Result<bool> {
        let key = PropertyIndexMetadata::edge_key(label, property);
        match self.txn.get(Db::Metadata, &key)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn load_edge(&self, edge_id: EdgeId) -> Result<Edge> {
        let bytes = self
            .txn
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
            .txn
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
