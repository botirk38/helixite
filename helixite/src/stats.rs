use std::collections::{BTreeMap, BTreeSet};

use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::index::properties::PropertyIndexRegistry;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub labels: Vec<LabelStats>,
    pub indexes: IndexStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelStats {
    pub label: String,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStats {
    pub node_properties: BTreeMap<String, Vec<String>>,
    pub edge_properties: BTreeMap<String, Vec<String>>,
}

impl GraphStats {
    pub(crate) fn load<S: StorageEngine>(storage: &S) -> Result<Self> {
        storage.read(|txn| {
            let mut labels = BTreeMap::<String, LabelCounts>::new();

            for entry in txn.iter(Db::Nodes, Scan::All)? {
                let entry = entry?;
                let node: Node = bincode::deserialize(entry.value)
                    .map_err(|e| HelixiteError::Codec(e.to_string()))?;
                labels.entry(node.label).or_default().node_count += 1;
            }

            for entry in txn.iter(Db::Edges, Scan::All)? {
                let entry = entry?;
                let edge: Edge = bincode::deserialize(entry.value)
                    .map_err(|e| HelixiteError::Codec(e.to_string()))?;
                labels.entry(edge.label).or_default().edge_count += 1;
            }

            let node_count = labels.values().map(LabelCounts::node_count).sum();
            let edge_count = labels.values().map(LabelCounts::edge_count).sum();
            let labels = labels.into_iter().map(LabelCounts::into_stats).collect();
            let indexes = IndexStats {
                node_properties: node_index_stats(txn)?,
                edge_properties: edge_index_stats(txn)?,
            };

            Ok(Self {
                node_count,
                edge_count,
                labels,
                indexes,
            })
        })
    }
}

#[derive(Default)]
struct LabelCounts {
    node_count: usize,
    edge_count: usize,
}

impl LabelCounts {
    fn node_count(&self) -> usize {
        self.node_count
    }

    fn edge_count(&self) -> usize {
        self.edge_count
    }

    fn into_stats((label, counts): (String, Self)) -> LabelStats {
        LabelStats {
            label,
            node_count: counts.node_count,
            edge_count: counts.edge_count,
        }
    }
}

fn indexed_properties(registry: PropertyIndexRegistry) -> BTreeMap<String, Vec<String>> {
    registry
        .into_indexes()
        .into_iter()
        .map(indexed_property_entry)
        .collect()
}

fn indexed_property_entry(
    (label, properties): (String, BTreeSet<String>),
) -> (String, Vec<String>) {
    (label, properties.into_iter().collect())
}

fn node_index_stats(txn: &dyn crate::storage::ReadTxn) -> Result<BTreeMap<String, Vec<String>>> {
    let registry = PropertyIndexRegistry::load_nodes_from_txn(txn)?;
    Ok(indexed_properties(registry))
}

fn edge_index_stats(txn: &dyn crate::storage::ReadTxn) -> Result<BTreeMap<String, Vec<String>>> {
    let registry = PropertyIndexRegistry::load_edges_from_txn(txn)?;
    Ok(indexed_properties(registry))
}
