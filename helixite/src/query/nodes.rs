use crate::error::Result;
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::value::Value;

use crate::index::labels::LabelIndex;
use crate::index::properties::PropertyIndex;

#[derive(Debug, Clone)]
pub(crate) enum PropertyFilter {
    Eq(String, Value),
}

pub struct NodeQuery<'a, S: StorageEngine> {
    storage: &'a S,
    label: Option<String>,
    filters: Vec<PropertyFilter>,
}

impl<'a, S: StorageEngine> NodeQuery<'a, S> {
    pub fn new(storage: &'a S) -> Self {
        Self {
            storage,
            label: None,
            filters: Vec::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn where_eq(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Eq(property.into(), value));
        self
    }

    pub fn collect(self) -> Result<Vec<Node>> {
        if self.filters.is_empty() {
            return self.collect_by_label();
        }

        let mut id_sets: Vec<Vec<NodeId>> = Vec::with_capacity(self.filters.len());
        for filter in &self.filters {
            let PropertyFilter::Eq(property, value) = filter;
            let ids = self.scan_ids_by_property(property, value)?;
            id_sets.push(ids);
        }

        let mut matching_ids = self.intersect_id_sets(id_sets)?;

        if let Some(ref label) = self.label {
            let label_ids = self.scan_ids_by_label(label)?;
            matching_ids.retain(|id| label_ids.binary_search(id).is_ok());
        }

        self.load_nodes(&matching_ids)
    }

    pub fn ids(self) -> Result<Vec<NodeId>> {
        self.collect()
            .map(|nodes| nodes.iter().map(|n| n.id).collect())
    }

    pub fn count(self) -> Result<usize> {
        if self.filters.is_empty() {
            return self.count_by_label();
        }

        let mut id_sets: Vec<Vec<NodeId>> = Vec::with_capacity(self.filters.len());
        for filter in &self.filters {
            let PropertyFilter::Eq(property, value) = filter;
            let ids = self.scan_ids_by_property(property, value)?;
            id_sets.push(ids);
        }

        let matching_ids = self.intersect_id_sets(id_sets)?;

        if let Some(ref label) = self.label {
            let label_ids = self.scan_ids_by_label(label)?;
            let count = matching_ids
                .iter()
                .filter(|id| label_ids.binary_search(id).is_ok())
                .count();
            return Ok(count);
        }

        Ok(matching_ids.len())
    }

    fn scan_ids_by_property(&self, property: &str, value: &Value) -> Result<Vec<NodeId>> {
        let Some(prefix) = PropertyIndex::prefix(property, value) else {
            return Ok(Vec::new());
        };

        let entries = self.storage.scan_prefix(Db::Properties, &prefix)?;
        let mut ids = Vec::new();

        for (key, _) in entries {
            if let Some(node_id) = PropertyIndex::decode_node_id(&key) {
                ids.push(node_id);
            }
        }

        ids.sort_unstable();
        Ok(ids)
    }

    fn scan_ids_by_label(&self, label: &str) -> Result<Vec<NodeId>> {
        let prefix = LabelIndex::prefix(label);
        let entries = self.storage.scan_prefix(Db::Labels, &prefix)?;
        let mut ids = Vec::new();

        for (key, _) in entries {
            if let Some(node_id) = LabelIndex::decode_node_id(&key) {
                ids.push(node_id);
            }
        }

        ids.sort_unstable();
        Ok(ids)
    }

    fn intersect_id_sets(&self, mut sets: Vec<Vec<NodeId>>) -> Result<Vec<NodeId>> {
        if sets.is_empty() {
            return Ok(Vec::new());
        }

        sets.sort_unstable_by_key(|s| s.len());

        let mut result = sets[0].clone();
        for set in &sets[1..] {
            if result.is_empty() {
                return Ok(Vec::new());
            }
            result.retain(|id| set.binary_search(id).is_ok());
        }

        Ok(result)
    }

    fn load_nodes(&self, ids: &[NodeId]) -> Result<Vec<Node>> {
        let mut nodes = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(bytes) = self.storage.get(Db::Nodes, &id.to_be_bytes())? else {
                continue;
            };
            let node: Node = bincode::deserialize(&bytes)
                .map_err(|e| crate::error::HelixiteError::Codec(e.to_string()))?;
            nodes.push(node);
        }
        Ok(nodes)
    }

    fn count_by_label(self) -> Result<usize> {
        let Some(ref label) = self.label else {
            return Ok(0);
        };

        let ids = self.scan_ids_by_label(label)?;
        Ok(ids.len())
    }

    fn collect_by_label(self) -> Result<Vec<Node>> {
        let ids = self.scan_ids_by_label(self.label.as_ref().unwrap())?;
        self.load_nodes(&ids)
    }
}
