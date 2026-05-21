use std::collections::BTreeSet;

use crate::edge::{Direction, Edge};
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::index::edges::EdgeIndex;
use crate::index::properties::{EdgePropertyIndex, PropertyIndexMetadata};
use crate::node::Node;
use crate::query::pagination::{Cursor, Page};
use crate::query::traversal::MultiHopTraversalQuery;
use crate::storage::ReadTxn;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};
use crate::value::Value;

#[derive(Debug, Clone)]
pub(crate) enum EdgePropertyFilter {
    Eq(String, Value),
}

pub struct NodeRefQuery<'a, S: StorageEngine> {
    storage: &'a S,
    node_id: NodeId,
}

pub struct TraversalQuery<'a, S: StorageEngine> {
    pub(super) storage: &'a S,
    pub(super) node_id: NodeId,
    pub(super) direction: Direction,
    pub(super) label: Option<String>,
    filters: Vec<EdgePropertyFilter>,
    pub(super) limit: Option<usize>,
    after: Option<String>,
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
            after: None,
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
            after: None,
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
            after: None,
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
            after: None,
        }
    }

    pub fn then_outgoing(self, label: impl Into<String>) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::new(self.storage, vec![self.node_id]).then_outgoing(label)
    }

    pub fn then_incoming(self, label: impl Into<String>) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::new(self.storage, vec![self.node_id]).then_incoming(label)
    }

    pub fn then_outgoing_any(self) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::new(self.storage, vec![self.node_id]).then_outgoing_any()
    }

    pub fn then_incoming_any(self) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::new(self.storage, vec![self.node_id]).then_incoming_any()
    }
}

impl<'a, S: StorageEngine> TraversalQuery<'a, S> {
    pub fn eq(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(EdgePropertyFilter::Eq(property.into(), value));
        self
    }

    pub fn then_outgoing(self, label: impl Into<String>) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::from_traversal(self).then_outgoing(label)
    }

    pub fn then_incoming(self, label: impl Into<String>) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::from_traversal(self).then_incoming(label)
    }

    pub fn then_outgoing_any(self) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::from_traversal(self).then_outgoing_any()
    }

    pub fn then_incoming_any(self) -> MultiHopTraversalQuery<'a, S> {
        MultiHopTraversalQuery::from_traversal(self).then_incoming_any()
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
        if self.after.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "after() requires page()".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: self.limit,
                after: None,
            };
            exec.collect_edges()
        })
    }

    pub fn nodes(self) -> Result<Vec<Node>> {
        if self.after.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "after() requires page()".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: self.limit,
                after: None,
            };
            exec.collect_nodes()
        })
    }

    pub fn count(self) -> Result<usize> {
        if self.after.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "after() requires page()".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: None,
                after: None,
            };
            exec.count()
        })
    }

    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    pub fn edges_page(self, size: usize) -> Result<Page<Edge>> {
        if self.limit.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "limit() cannot be used with page()".into(),
            ));
        }
        if size == 0 {
            return Err(HelixiteError::InvalidConfig(
                "page size must be greater than 0".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: None,
                after: self.after,
            };
            exec.page(size)
        })
    }

    pub fn nodes_page(self, size: usize) -> Result<Page<Node>> {
        if self.limit.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "limit() cannot be used with page()".into(),
            ));
        }
        if size == 0 {
            return Err(HelixiteError::InvalidConfig(
                "page size must be greater than 0".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = TraversalExec {
                txn,
                node_id: self.node_id,
                direction: self.direction,
                label: self.label,
                filters: self.filters,
                limit: None,
                after: self.after,
            };
            exec.nodes_page(size)
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
    after: Option<String>,
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

    fn resolve_ordered_edge_ids(&self) -> Result<Vec<EdgeId>> {
        if !self.filters.is_empty() {
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

            let mut ids: Vec<EdgeId> = candidate_ids
                .into_iter()
                .filter(|id| adjacency_ids.contains(id))
                .collect();
            if let Some(limit) = self.limit {
                ids.truncate(limit);
            }
            return Ok(ids);
        }

        let (db, prefix) = self.prefix_and_db();
        let mut ids = Vec::new();

        for entry in self.txn.iter(db, Scan::Prefix(&prefix))? {
            let entry = entry?;
            let edge_id = EdgeIndex::decode_edge_id(entry.key)
                .ok_or_else(|| HelixiteError::Storage("corrupt edge adjacency key".into()))?;
            ids.push(edge_id);
            if let Some(limit) = self.limit
                && ids.len() >= limit
            {
                break;
            }
        }

        Ok(ids)
    }

    fn page(self, page_size: usize) -> Result<Page<Edge>> {
        let ordered_ids = self.resolve_ordered_edge_ids()?;

        let after = self
            .after
            .as_ref()
            .map(|s| Cursor::decode_edge(s))
            .transpose()?;

        let page = Page::from_iter(
            ordered_ids,
            page_size,
            after.as_ref(),
            |id| after.as_ref().is_some_and(|c| c.matches_edge(*id)),
            |id| Cursor::encode_edge(*id),
        )?;

        let edges = page
            .items
            .into_iter()
            .map(|id| self.load_edge(id))
            .collect::<Result<Vec<_>>>()?;

        Ok(Page {
            items: edges,
            next_cursor: page.next_cursor,
        })
    }

    fn nodes_page(self, page_size: usize) -> Result<Page<Node>> {
        let ordered_ids = self.resolve_ordered_edge_ids()?;

        let after = self
            .after
            .as_ref()
            .map(|s| Cursor::decode_edge(s))
            .transpose()?;

        let page = Page::from_iter(
            ordered_ids,
            page_size,
            after.as_ref(),
            |id| after.as_ref().is_some_and(|c| c.matches_edge(*id)),
            |id| Cursor::encode_edge(*id),
        )?;

        let mut nodes = Vec::with_capacity(page.items.len());
        for edge_id in page.items {
            let edge = self.load_edge(edge_id)?;
            let target_id = EdgeIndex::decode_target_from_edge(&edge, self.direction);
            let node = self.load_node(target_id)?;
            nodes.push(node);
        }

        Ok(Page {
            items: nodes,
            next_cursor: page.next_cursor,
        })
    }
}
