use std::collections::BTreeMap;

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

            let node_count = labels.values().map(|counts| counts.node_count).sum();
            let edge_count = labels.values().map(|counts| counts.edge_count).sum();
            let labels = labels
                .into_iter()
                .map(
                    |(
                        label,
                        LabelCounts {
                            node_count,
                            edge_count,
                        },
                    )| LabelStats {
                        label,
                        node_count,
                        edge_count,
                    },
                )
                .collect();

            Ok(Self {
                node_count,
                edge_count,
                labels,
                indexes: IndexStats {
                    node_properties: indexed_properties(
                        PropertyIndexRegistry::load_nodes_from_txn(txn)?,
                    ),
                    edge_properties: indexed_properties(
                        PropertyIndexRegistry::load_edges_from_txn(txn)?,
                    ),
                },
            })
        })
    }
}

#[derive(Default)]
struct LabelCounts {
    node_count: usize,
    edge_count: usize,
}

fn indexed_properties(registry: PropertyIndexRegistry) -> BTreeMap<String, Vec<String>> {
    registry
        .into_indexes()
        .into_iter()
        .map(|(label, properties)| (label, properties.into_iter().collect()))
        .collect()
}
