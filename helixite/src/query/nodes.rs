use crate::error::{HelixiteError, Result};
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
    pub(crate) fn new(storage: &'a S) -> Self {
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
        let matching_ids = self.resolve_matching_ids()?;
        self.load_nodes(&matching_ids)
    }

    pub fn ids(self) -> Result<Vec<NodeId>> {
        self.resolve_matching_ids()
    }

    pub fn count(self) -> Result<usize> {
        let matching_ids = self.resolve_matching_ids()?;
        Ok(matching_ids.len())
    }

    fn resolve_matching_ids(&self) -> Result<Vec<NodeId>> {
        if self.filters.is_empty() {
            return match &self.label {
                Some(label) => self.scan_ids_by_label(label),
                None => self.scan_all_node_ids(),
            };
        }

        let mut id_sets: Vec<Vec<NodeId>> = Vec::with_capacity(self.filters.len());
        for filter in &self.filters {
            let PropertyFilter::Eq(property, value) = filter;
            id_sets.push(self.scan_ids_by_property(property, value)?);
        }

        let mut matching_ids = self.intersect_id_sets(id_sets)?;

        if let Some(ref label) = self.label {
            let label_ids = self.scan_ids_by_label(label)?;
            matching_ids.retain(|id| label_ids.binary_search(id).is_ok());
        }

        Ok(matching_ids)
    }

    fn scan_ids_by_property(&self, property: &str, value: &Value) -> Result<Vec<NodeId>> {
        let Some(prefix) = PropertyIndex::prefix(property, value) else {
            return Ok(Vec::new());
        };

        let entries = self.storage.scan_prefix(Db::Properties, &prefix)?;
        let mut ids = Vec::new();

        for (key, _) in entries {
            let node_id = PropertyIndex::decode_node_id(&key)
                .ok_or_else(|| HelixiteError::Storage("corrupt property index key".into()))?;
            ids.push(node_id);
        }

        ids.sort_unstable();
        Ok(ids)
    }

    fn scan_ids_by_label(&self, label: &str) -> Result<Vec<NodeId>> {
        let prefix = LabelIndex::prefix(label);
        let entries = self.storage.scan_prefix(Db::Labels, &prefix)?;
        let mut ids = Vec::new();

        for (key, _) in entries {
            let node_id = LabelIndex::decode_node_id(&key)
                .ok_or_else(|| HelixiteError::Storage("corrupt label index key".into()))?;
            ids.push(node_id);
        }

        ids.sort_unstable();
        Ok(ids)
    }

    fn scan_all_node_ids(&self) -> Result<Vec<NodeId>> {
        let entries = self.storage.scan_prefix(Db::Nodes, &[])?;
        let mut ids = Vec::new();

        for (key, _) in entries {
            let id_bytes: [u8; 8] = key
                .try_into()
                .map_err(|_| HelixiteError::Storage("corrupt node key".into()))?;
            ids.push(u64::from_be_bytes(id_bytes));
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
            let bytes = self
                .storage
                .get(Db::Nodes, &id.to_be_bytes())?
                .ok_or(HelixiteError::NodeNotFound(*id))?;
            let node: Node =
                bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            nodes.push(node);
        }
        Ok(nodes)
    }
}
