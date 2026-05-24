use std::collections::{BTreeMap, BTreeSet};

use crate::edge::Edge;
use crate::error::{IvyError, Result};
use crate::index::properties::PropertyIndexRegistry;
use crate::node::Node;
use crate::storage::engine::{Db, Scan};
use crate::storage::{ReadTxn, StorageEngine};

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
                    .map_err(|e| IvyError::Codec(e.to_string()))?;
                labels.entry(node.label).or_default().node_count += 1;
            }

            for entry in txn.iter(Db::Edges, Scan::All)? {
                let entry = entry?;
                let edge: Edge = bincode::deserialize(entry.value)
                    .map_err(|e| IvyError::Codec(e.to_string()))?;
                labels.entry(edge.label).or_default().edge_count += 1;
            }

            let node_count = labels.values().map(LabelCounts::node_count).sum();
            let edge_count = labels.values().map(LabelCounts::edge_count).sum();
            let labels = labels.into_iter().map(LabelCounts::into_stats).collect();
            let indexes = IndexStats {
                node_properties: IndexStats::node_properties(txn)?,
                edge_properties: IndexStats::edge_properties(txn)?,
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

impl IndexStats {
    fn node_properties(txn: &dyn ReadTxn) -> Result<BTreeMap<String, Vec<String>>> {
        let registry = PropertyIndexRegistry::load_nodes_from_txn(txn)?;
        Ok(Self::properties(registry))
    }

    fn edge_properties(txn: &dyn ReadTxn) -> Result<BTreeMap<String, Vec<String>>> {
        let registry = PropertyIndexRegistry::load_edges_from_txn(txn)?;
        Ok(Self::properties(registry))
    }

    fn properties(registry: PropertyIndexRegistry) -> BTreeMap<String, Vec<String>> {
        registry
            .into_indexes()
            .into_iter()
            .map(Self::property_entry)
            .collect()
    }

    fn property_entry((label, properties): (String, BTreeSet<String>)) -> (String, Vec<String>) {
        (label, properties.into_iter().collect())
    }
}
