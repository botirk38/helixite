use std::collections::BTreeSet;

use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::EdgeId;
use crate::index::labels::EdgeLabelIndex;
use crate::index::properties::EdgePropertyIndex;
use crate::index::properties::PropertyIndexMetadata;
use crate::query::filter::PropertyFilter;
use crate::query::pagination::{Cursor, Page};
use crate::storage::ReadTxn;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};
use crate::value::Value;

pub struct EdgeQuery<'a, S: StorageEngine> {
    storage: &'a S,
    label: Option<String>,
    filters: Vec<PropertyFilter>,
    limit: Option<usize>,
    after: Option<String>,
}

impl<'a, S: StorageEngine> EdgeQuery<'a, S> {
    pub(crate) fn new(storage: &'a S) -> Self {
        Self {
            storage,
            label: None,
            filters: Vec::new(),
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

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn first(self) -> Result<Option<Edge>> {
        let edges = self.limit(1).collect()?;
        Ok(edges.into_iter().next())
    }

    pub fn collect(self) -> Result<Vec<Edge>> {
        if self.after.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "after() requires page()".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = EdgeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                limit: self.limit,
                after: None,
            };
            exec.collect()
        })
    }

    pub fn ids(self) -> Result<Vec<EdgeId>> {
        if self.after.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "after() requires page()".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = EdgeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                limit: self.limit,
                after: None,
            };
            exec.resolve_ordered_ids()
        })
    }

    pub fn count(self) -> Result<usize> {
        if self.after.is_some() {
            return Err(HelixiteError::InvalidConfig(
                "after() requires page()".into(),
            ));
        }
        self.storage.read(|txn| {
            let exec = EdgeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
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

    pub fn page(self, size: usize) -> Result<Page<Edge>> {
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
            let exec = EdgeQueryExec {
                txn,
                label: self.label,
                filters: self.filters,
                limit: None,
                after: self.after,
            };
            exec.page(size)
        })
    }
}

struct EdgeQueryExec<'a> {
    txn: &'a dyn ReadTxn,
    label: Option<String>,
    filters: Vec<PropertyFilter>,
    limit: Option<usize>,
    after: Option<String>,
}

impl EdgeQueryExec<'_> {
    fn collect(self) -> Result<Vec<Edge>> {
        let ids = self.resolve_ordered_ids()?;
        self.load_edges(&ids)
    }

    fn resolve_ordered_ids(&self) -> Result<Vec<EdgeId>> {
        let mut ids = if self.filters.is_empty() {
            if let Some(ref label) = self.label {
                self.scan_label_ids(label)?
            } else {
                self.scan_all_edge_ids()?
            }
        } else {
            self.resolve_filter_ids()?.into_iter().collect()
        };

        if let Some(limit) = self.limit {
            ids.truncate(limit);
        }

        Ok(ids)
    }

    fn resolve_filter_ids(&self) -> Result<BTreeSet<EdgeId>> {
        let label = self.label.as_ref().ok_or_else(|| {
            HelixiteError::IndexNotFound("edge property filter requires a label".to_string())
        })?;

        let mut sets: Vec<BTreeSet<EdgeId>> = Vec::with_capacity(self.filters.len() + 1);

        let label_ids: BTreeSet<EdgeId> = self.scan_label_ids(label)?.into_iter().collect();
        sets.push(label_ids);

        for filter in &self.filters {
            let property = filter.property();

            if !self.is_property_indexed(label, property)? {
                return Err(HelixiteError::IndexNotFound(format!(
                    "no edge property index for {label}::{property}"
                )));
            }

            let mut set = BTreeSet::new();
            for entry in self.txn.iter(
                Db::Properties,
                Scan::Prefix(&EdgePropertyIndex::index_prefix(label, property)),
            )? {
                let entry = entry?;
                let edge_id = EdgePropertyIndex::decode_edge_id(entry.key)
                    .ok_or_else(|| HelixiteError::Storage("corrupt property index key".into()))?;
                let indexed_value = EdgePropertyIndex::decode_value(entry.key)
                    .ok_or_else(|| HelixiteError::Storage("corrupt property index key".into()))?;
                if filter.matches_indexed(&indexed_value) {
                    set.insert(edge_id);
                }
            }
            sets.push(set);
        }

        let mut result = sets.first().cloned().unwrap_or_default();
        for set in &sets[1..] {
            result = result.intersection(set).copied().collect();
        }
        Ok(result)
    }

    fn is_property_indexed(&self, label: &str, property: &str) -> Result<bool> {
        let key = PropertyIndexMetadata::edge_key(label, property);
        match self.txn.get(Db::Metadata, &key)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn scan_label_ids(&self, label: &str) -> Result<Vec<EdgeId>> {
        let prefix = EdgeLabelIndex::prefix(label);
        let mut ids = Vec::new();

        for entry in self.txn.iter(Db::Labels, Scan::Prefix(&prefix))? {
            let entry = entry?;
            let edge_id = EdgeLabelIndex::decode_edge_id(entry.key)
                .ok_or_else(|| HelixiteError::Storage("corrupt edge label index key".into()))?;
            ids.push(edge_id);
        }
        ids.sort_unstable();
        Ok(ids)
    }

    fn scan_all_edge_ids(&self) -> Result<Vec<EdgeId>> {
        let mut ids = Vec::new();
        for entry in self.txn.iter(Db::Edges, Scan::All)? {
            let entry = entry?;
            let id_bytes: [u8; 8] = entry
                .key
                .try_into()
                .map_err(|_| HelixiteError::Storage("corrupt edge key".into()))?;
            ids.push(u64::from_be_bytes(id_bytes));
        }
        ids.sort_unstable();
        Ok(ids)
    }

    fn load_edges(&self, ids: &[EdgeId]) -> Result<Vec<Edge>> {
        ids.iter().map(|id| self.load_edge(*id)).collect()
    }

    fn load_edge(&self, edge_id: EdgeId) -> Result<Edge> {
        let bytes = self
            .txn
            .get(Db::Edges, &edge_id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(edge_id))?;
        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    fn page(self, page_size: usize) -> Result<Page<Edge>> {
        let ordered_ids = self.resolve_ordered_ids()?;

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
}
