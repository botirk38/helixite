use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::value::Value;

use crate::index::labels::LabelIndex;
use crate::index::properties::NodePropertyIndex;
use crate::index::properties::PropertyIndexMetadata;
use crate::index::vector::VectorIndex;

#[derive(Debug, Clone)]
pub(crate) enum PropertyFilter {
    Eq(String, Value),
}

pub struct NodeQuery<'a, S: StorageEngine> {
    storage: &'a S,
    label: Option<String>,
    filters: Vec<PropertyFilter>,
    vector_search: Option<VectorSearch>,
}

#[derive(Debug, Clone)]
struct VectorSearch {
    property: String,
    query: Vec<f32>,
    k: usize,
}

impl<'a, S: StorageEngine> NodeQuery<'a, S> {
    pub(crate) fn new(storage: &'a S) -> Self {
        Self {
            storage,
            label: None,
            filters: Vec::new(),
            vector_search: None,
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

    pub fn nearest(mut self, property: impl Into<String>, query: Vec<f32>, k: usize) -> Self {
        self.vector_search = Some(VectorSearch {
            property: property.into(),
            query,
            k,
        });
        self
    }

    pub fn collect(self) -> Result<Vec<Node>> {
        let ordered_ids = self.resolve_ordered_ids()?;
        self.load_nodes(&ordered_ids)
    }

    pub fn ids(self) -> Result<Vec<NodeId>> {
        self.resolve_ordered_ids()
    }

    pub fn count(self) -> Result<usize> {
        let matching_ids = self.resolve_ordered_ids()?;
        Ok(matching_ids.len())
    }

    fn resolve_ordered_ids(&self) -> Result<Vec<NodeId>> {
        if let Some(ref vs) = self.vector_search {
            let label = self
                .label
                .as_ref()
                .ok_or_else(|| HelixiteError::Storage("vector search requires a label".into()))?;

            let meta = VectorIndex::load_meta(self.storage, label, &vs.property)?;
            let search_k = if self.filters.is_empty() {
                vs.k
            } else {
                vs.k * 10
            };
            let mut results = VectorIndex::search(
                self.storage,
                label,
                &vs.property,
                &vs.query,
                search_k,
                &meta,
            )?;

            if !self.filters.is_empty() {
                let filter_ids = self.resolve_filter_ids()?;
                let filter_set: std::collections::HashSet<NodeId> =
                    filter_ids.into_iter().collect();
                results.retain(|(id, _)| filter_set.contains(id));
            }

            results.truncate(vs.k);

            return Ok(results.into_iter().map(|(id, _)| id).collect());
        }

        if self.filters.is_empty() {
            return match &self.label {
                Some(label) => self.scan_ids_by_label(label),
                None => self.scan_all_node_ids(),
            };
        }

        let mut matching_ids = self.resolve_filter_ids()?;

        if let Some(ref label) = self.label {
            let label_ids = self.scan_ids_by_label(label)?;
            matching_ids.retain(|id| label_ids.binary_search(id).is_ok());
        }

        Ok(matching_ids)
    }

    fn resolve_filter_ids(&self) -> Result<Vec<NodeId>> {
        let label = self.label.as_ref().ok_or_else(|| {
            HelixiteError::IndexNotFound("property filter requires a label".to_string())
        })?;

        let mut id_sets: Vec<Vec<NodeId>> = Vec::with_capacity(self.filters.len());
        for filter in &self.filters {
            let PropertyFilter::Eq(property, value) = filter;

            if !self.is_property_indexed(label, property)? {
                return Err(HelixiteError::IndexNotFound(format!(
                    "no property index for {label}::{property}"
                )));
            }

            id_sets.push(self.scan_ids_by_property(property, value, label)?);
        }
        self.intersect_id_sets(id_sets)
    }

    fn is_property_indexed(&self, label: &str, property: &str) -> Result<bool> {
        let key = PropertyIndexMetadata::node_key(label, property);
        match self.storage.get(Db::Metadata, &key)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn scan_ids_by_property(
        &self,
        property: &str,
        value: &Value,
        label: &str,
    ) -> Result<Vec<NodeId>> {
        let Some(prefix) = NodePropertyIndex::lookup_prefix(label, property, value) else {
            return Ok(Vec::new());
        };

        let entries = self.storage.scan_prefix(Db::Properties, &prefix)?;
        let mut ids = Vec::new();

        for (key, _) in entries {
            let node_id = NodePropertyIndex::decode_node_id(&key)
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
