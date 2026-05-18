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

        let PropertyFilter::Eq(property, value) = &self.filters[0];
        let mut nodes = self.scan_by_property(property, value)?;

        for filter in &self.filters[1..] {
            let PropertyFilter::Eq(prop, val) = filter;
            nodes.retain(|n| n.properties.get(prop) == Some(val));
        }

        if let Some(ref label) = self.label {
            nodes.retain(|n| &n.label == label);
        }

        Ok(nodes)
    }

    pub fn ids(self) -> Result<Vec<NodeId>> {
        self.collect()
            .map(|nodes| nodes.iter().map(|n| n.id).collect())
    }

    pub fn count(self) -> Result<usize> {
        self.collect().map(|nodes| nodes.len())
    }

    fn scan_by_property(&self, property: &str, value: &Value) -> Result<Vec<Node>> {
        let Some(prefix) = PropertyIndex::prefix(property, value) else {
            return Ok(Vec::new());
        };

        let entries = self.storage.scan_prefix(Db::Properties, &prefix)?;
        let mut nodes = Vec::new();

        for (key, _) in entries {
            let Some(node_id) = PropertyIndex::decode_node_id(&key) else {
                continue;
            };
            let Some(bytes) = self.storage.get(Db::Nodes, &node_id.to_be_bytes())? else {
                continue;
            };
            let node: Node = bincode::deserialize(&bytes)
                .map_err(|e| crate::error::HelixiteError::Codec(e.to_string()))?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    fn collect_by_label(self) -> Result<Vec<Node>> {
        let Some(ref label) = self.label else {
            return Ok(Vec::new());
        };

        let prefix = LabelIndex::prefix(label);
        let entries = self.storage.scan_prefix(Db::Labels, &prefix)?;
        let mut nodes = Vec::new();

        for (key, _) in entries {
            let Some(node_id) = LabelIndex::decode_node_id(&key) else {
                continue;
            };
            let Some(bytes) = self.storage.get(Db::Nodes, &node_id.to_be_bytes())? else {
                continue;
            };
            let node: Node = bincode::deserialize(&bytes)
                .map_err(|e| crate::error::HelixiteError::Codec(e.to_string()))?;
            nodes.push(node);
        }

        Ok(nodes)
    }
}
