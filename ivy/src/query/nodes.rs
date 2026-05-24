use crate::error::{IvyError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::query::filter::PropertyFilter;
use crate::query::pagination::{Cursor, Page};
use crate::storage::ReadTxn;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};
use crate::value::Value;

use crate::index::labels::NodeLabelIndex;
use crate::index::properties::NodePropertyIndex;
use crate::index::properties::PropertyIndexRegistry;
use crate::index::vector::VectorIndex;

pub struct NodeQuery<'a, S: StorageEngine> {
    storage: &'a S,
    label: Option<String>,
    filters: Vec<PropertyFilter>,
    vector_search: Option<VectorSearch>,
    limit: Option<usize>,
    after: Option<String>,
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
            limit: None,
            after: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn eq(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Eq(property.into(), value));
        self
    }

    pub fn ne(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Ne(property.into(), value));
        self
    }

    pub fn gt(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Gt(property.into(), value));
        self
    }

    pub fn gte(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Gte(property.into(), value));
        self
    }

    pub fn lt(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Lt(property.into(), value));
        self
    }

    pub fn lte(mut self, property: impl Into<String>, value: Value) -> Self {
        self.filters
            .push(PropertyFilter::Lte(property.into(), value));
        self
    }

    pub fn r#in(
        mut self,
        property: impl Into<String>,
        values: impl IntoIterator<Item = Value>,
    ) -> Self {
        self.filters.push(PropertyFilter::In(
            property.into(),
            values.into_iter().collect(),
        ));
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

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn first(self) -> Result<Option<Node>> {
        let nodes = self.limit(1).collect()?;
        Ok(nodes.into_iter().next())
    }

    pub fn collect(self) -> Result<Vec<Node>> {
        if self.after.is_some() {
            return Err(IvyError::InvalidConfig("after() requires page()".into()));
        }
        self.storage.read(|txn| {
            let exec = NodeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                vector_search: self.vector_search,
                limit: self.limit,
                after: None,
            };
            exec.collect()
        })
    }

    pub fn ids(self) -> Result<Vec<NodeId>> {
        if self.after.is_some() {
            return Err(IvyError::InvalidConfig("after() requires page()".into()));
        }
        self.storage.read(|txn| {
            let exec = NodeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                vector_search: self.vector_search,
                limit: self.limit,
                after: None,
            };
            exec.resolve_ordered_ids()
        })
    }

    pub fn count(self) -> Result<usize> {
        if self.after.is_some() {
            return Err(IvyError::InvalidConfig("after() requires page()".into()));
        }
        self.storage.read(|txn| {
            let exec = NodeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                vector_search: self.vector_search,
                limit: None,
                after: None,
            };
            let matching_ids = exec.resolve_ordered_ids()?;
            Ok(matching_ids.len())
        })
    }

    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    pub fn page(self, size: usize) -> Result<Page<Node>> {
        if self.limit.is_some() {
            return Err(IvyError::InvalidConfig(
                "limit() cannot be used with page()".into(),
            ));
        }
        if size == 0 {
            return Err(IvyError::InvalidConfig(
                "page size must be greater than 0".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = NodeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                vector_search: self.vector_search,
                limit: None,
                after: self.after,
            };
            exec.page(size)
        })
    }
}

struct NodeQueryExec<'a> {
    txn: &'a dyn ReadTxn,
    label: Option<String>,
    filters: Vec<PropertyFilter>,
    vector_search: Option<VectorSearch>,
    limit: Option<usize>,
    after: Option<String>,
}

impl NodeQueryExec<'_> {
    fn collect(self) -> Result<Vec<Node>> {
        let ordered_ids = self.resolve_ordered_ids()?;
        self.load_nodes(&ordered_ids)
    }

    fn resolve_ordered_ids(&self) -> Result<Vec<NodeId>> {
        if let Some(ref vs) = self.vector_search {
            let label = self
                .label
                .as_ref()
                .ok_or_else(|| IvyError::Storage("vector search requires a label".into()))?;

            let meta = VectorIndex::load_meta(self.txn, label, &vs.property)?;
            let search_k = if self.filters.is_empty() {
                vs.k
            } else {
                vs.k * 10
            };
            let mut results =
                VectorIndex::search(self.txn, label, &vs.property, &vs.query, search_k, &meta)?;

            if !self.filters.is_empty() {
                let filter_ids = self.resolve_filter_ids()?;
                let filter_set: std::collections::HashSet<NodeId> =
                    filter_ids.into_iter().collect();
                results.retain(|(id, _)| filter_set.contains(id));
            }

            results.truncate(vs.k);

            let mut ids: Vec<NodeId> = results.into_iter().map(|(id, _)| id).collect();
            if let Some(limit) = self.limit {
                ids.truncate(limit);
            }
            return Ok(ids);
        }

        if self.filters.is_empty() {
            let mut result = match &self.label {
                Some(label) => self.scan_ids_by_label(label, self.limit)?,
                None => self.scan_all_node_ids(self.limit)?,
            };
            if let Some(limit) = self.limit {
                result.truncate(limit);
            }
            return Ok(result);
        }

        let mut matching_ids = self.resolve_filter_ids()?;

        if let Some(ref label) = self.label {
            let label_ids = self.scan_ids_by_label(label, None)?;
            matching_ids.retain(|id| label_ids.binary_search(id).is_ok());
        }

        let mut result = matching_ids;
        if let Some(limit) = self.limit {
            result.truncate(limit);
        }
        Ok(result)
    }

    fn resolve_filter_ids(&self) -> Result<Vec<NodeId>> {
        let label = self.label.as_ref().ok_or_else(|| {
            IvyError::IndexNotFound("property filter requires a label".to_string())
        })?;

        let registry = PropertyIndexRegistry::load_nodes_for_label(self.txn, label)?;
        let mut id_sets: Vec<Vec<NodeId>> = Vec::with_capacity(self.filters.len());
        for filter in &self.filters {
            let property = filter.property();

            if !registry.contains(label, property) {
                return Err(IvyError::IndexNotFound(format!(
                    "no property index for {label}::{property}"
                )));
            }

            id_sets.push(self.scan_ids_by_property(filter, label)?);
        }
        self.intersect_id_sets(id_sets)
    }

    fn scan_ids_by_property(&self, filter: &PropertyFilter, label: &str) -> Result<Vec<NodeId>> {
        let mut ids = Vec::new();

        for entry in self.txn.iter(
            Db::Properties,
            Scan::Prefix(&NodePropertyIndex::index_prefix(label, filter.property())),
        )? {
            let entry = entry?;
            let node_id = NodePropertyIndex::decode_node_id(entry.key)
                .ok_or_else(|| IvyError::Storage("corrupt property index key".into()))?;
            let indexed_value = NodePropertyIndex::decode_value(entry.key)
                .ok_or_else(|| IvyError::Storage("corrupt property index key".into()))?;
            if filter.matches_indexed(&indexed_value) {
                ids.push(node_id);
            }
        }

        ids.sort_unstable();
        Ok(ids)
    }

    fn scan_ids_by_label(&self, label: &str, limit: Option<usize>) -> Result<Vec<NodeId>> {
        let prefix = NodeLabelIndex::prefix(label);
        let mut ids = Vec::new();

        for entry in self.txn.iter(Db::Labels, Scan::Prefix(&prefix))? {
            let entry = entry?;
            let node_id = NodeLabelIndex::decode_node_id(entry.key)
                .ok_or_else(|| IvyError::Storage("corrupt label index key".into()))?;
            ids.push(node_id);
            if let Some(limit) = limit
                && ids.len() >= limit
            {
                break;
            }
        }

        ids.sort_unstable();
        Ok(ids)
    }

    fn scan_all_node_ids(&self, limit: Option<usize>) -> Result<Vec<NodeId>> {
        let mut ids = Vec::new();

        for entry in self.txn.iter(Db::Nodes, Scan::All)? {
            let entry = entry?;
            let id_bytes: [u8; 8] = entry
                .key
                .try_into()
                .map_err(|_| IvyError::Storage("corrupt node key".into()))?;
            ids.push(u64::from_be_bytes(id_bytes));
            if let Some(limit) = limit
                && ids.len() >= limit
            {
                break;
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
            let bytes = self
                .txn
                .get(Db::Nodes, &id.to_be_bytes())?
                .ok_or(IvyError::NodeNotFound(*id))?;
            let node: Node =
                bincode::deserialize(&bytes).map_err(|e| IvyError::Codec(e.to_string()))?;
            nodes.push(node);
        }
        Ok(nodes)
    }

    fn page(self, page_size: usize) -> Result<Page<Node>> {
        let ordered_ids = self.resolve_ordered_ids()?;

        let after = self
            .after
            .as_ref()
            .map(|s| Cursor::decode_node(s))
            .transpose()?;

        let page = Page::from_iter(
            ordered_ids,
            page_size,
            after.as_ref(),
            |id| after.as_ref().is_some_and(|c| c.matches_node(*id)),
            |id| Cursor::encode_node(*id),
        )?;

        let nodes = self.load_nodes(&page.items)?;
        Ok(Page {
            items: nodes,
            next_cursor: page.next_cursor,
        })
    }
}
