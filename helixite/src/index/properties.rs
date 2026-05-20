use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::storage::ReadTxn;
use crate::storage::StorageEngine;
use crate::storage::WriteTxn;
use crate::storage::engine::{Db, Scan};
use crate::value::{IndexedValue, Value};

use super::codec::{KeyBuilder, KeyReader};
use super::labels::NodeLabelIndex;

pub(crate) struct NodePropertyIndex;

impl NodePropertyIndex {
    pub(crate) fn key(
        label: &str,
        property: &str,
        value: &Value,
        node_id: NodeId,
    ) -> Option<Vec<u8>> {
        let value_bytes = value.to_index_key()?;
        Some(
            KeyBuilder::new()
                .u8(0)
                .str(label)
                .str(property)
                .bytes(&value_bytes)
                .u64(node_id)
                .finish(),
        )
    }

    pub(crate) fn index_prefix(label: &str, property: &str) -> Vec<u8> {
        KeyBuilder::new().u8(0).str(label).str(property).finish()
    }

    pub(crate) fn decode_node_id(key: &[u8]) -> Option<NodeId> {
        let mut reader = KeyReader::new(key);
        reader.u8()?;
        reader.str()?;
        reader.str()?;
        reader.bytes()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }

    pub(crate) fn decode_value(key: &[u8]) -> Option<IndexedValue> {
        let mut reader = KeyReader::new(key);
        reader.u8()?;
        reader.str()?;
        reader.str()?;
        Value::from_index_key(reader.bytes()?)
    }
}

pub(crate) struct EdgePropertyIndex;

impl EdgePropertyIndex {
    pub(crate) fn key(
        label: &str,
        property: &str,
        value: &Value,
        edge_id: EdgeId,
    ) -> Option<Vec<u8>> {
        let value_bytes = value.to_index_key()?;
        Some(
            KeyBuilder::new()
                .u8(1)
                .str(label)
                .str(property)
                .bytes(&value_bytes)
                .u64(edge_id)
                .finish(),
        )
    }

    pub(crate) fn index_prefix(label: &str, property: &str) -> Vec<u8> {
        KeyBuilder::new().u8(1).str(label).str(property).finish()
    }

    pub(crate) fn decode_edge_id(key: &[u8]) -> Option<EdgeId> {
        let mut reader = KeyReader::new(key);
        reader.u8()?;
        reader.str()?;
        reader.str()?;
        reader.bytes()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }

    pub(crate) fn decode_value(key: &[u8]) -> Option<IndexedValue> {
        let mut reader = KeyReader::new(key);
        reader.u8()?;
        reader.str()?;
        reader.str()?;
        Value::from_index_key(reader.bytes()?)
    }
}

pub(crate) struct PropertyIndexMetadata;

impl PropertyIndexMetadata {
    pub(crate) fn node_key(label: &str, property: &str) -> Vec<u8> {
        KeyBuilder::new().u8(0).str(label).str(property).finish()
    }

    pub(crate) fn edge_key(label: &str, property: &str) -> Vec<u8> {
        KeyBuilder::new().u8(1).str(label).str(property).finish()
    }

    pub(crate) fn node_prefix() -> Vec<u8> {
        KeyBuilder::new().u8(0).finish()
    }

    pub(crate) fn edge_prefix() -> Vec<u8> {
        KeyBuilder::new().u8(1).finish()
    }

    pub(crate) fn decode_label_property(key: &[u8]) -> Option<(String, String)> {
        let mut reader = KeyReader::new(key);
        reader.u8()?;
        let label = std::str::from_utf8(reader.str()?).ok()?.to_string();
        let property = std::str::from_utf8(reader.str()?).ok()?.to_string();
        reader.finish()?;
        Some((label, property))
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PropertyIndexRegistry {
    indexes: BTreeMap<String, BTreeSet<String>>,
}

impl PropertyIndexRegistry {
    pub(crate) fn contains(&self, label: &str, property: &str) -> bool {
        self.indexes
            .get(label)
            .is_some_and(|props| props.contains(property))
    }

    pub(crate) fn load_nodes_from_txn(txn: &dyn ReadTxn) -> crate::error::Result<Self> {
        Self::load_from_txn(txn, PropertyIndexMetadata::node_prefix())
    }

    pub(crate) fn load_edges_from_txn(txn: &dyn ReadTxn) -> crate::error::Result<Self> {
        Self::load_from_txn(txn, PropertyIndexMetadata::edge_prefix())
    }

    fn load_from_txn(txn: &dyn ReadTxn, prefix: Vec<u8>) -> crate::error::Result<Self> {
        let mut indexes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for entry in txn.iter(Db::Metadata, Scan::Prefix(&prefix))? {
            let entry = entry?;
            let Some((label, prop)) = PropertyIndexMetadata::decode_label_property(entry.key)
            else {
                continue;
            };

            indexes.entry(label).or_default().insert(prop);
        }

        Ok(Self { indexes })
    }
}

pub(crate) struct NodePropertyIndexes;

impl NodePropertyIndexes {
    pub(crate) fn create(storage: &impl StorageEngine, label: &str, property: &str) -> Result<()> {
        let metadata_key = PropertyIndexMetadata::node_key(label, property);

        storage.write(|txn| {
            if !Self::label_exists(txn, label)? {
                return Err(HelixiteError::LabelNotFound(label.to_string()));
            }

            if txn.get(Db::Metadata, &metadata_key)?.is_some() {
                return Err(HelixiteError::DuplicateKey(format!(
                    "node property index {label}::{property} already exists"
                )));
            }

            Self::backfill(txn, label, property)?;
            txn.put(Db::Metadata, &metadata_key, &[])
        })
    }

    pub(crate) fn drop(storage: &impl StorageEngine, label: &str, property: &str) -> Result<()> {
        let metadata_key = PropertyIndexMetadata::node_key(label, property);

        storage.write(|txn| {
            if txn.get(Db::Metadata, &metadata_key)?.is_none() {
                return Err(HelixiteError::IndexNotFound(format!(
                    "node property index {label}::{property}"
                )));
            }

            let prefix = NodePropertyIndex::index_prefix(label, property);
            let entries = txn.scan(Db::Properties, Scan::Prefix(&prefix), None)?;
            let keys: Vec<Vec<u8>> = entries.iter().map(|e| e.key.to_vec()).collect();
            for key in keys {
                txn.delete(Db::Properties, &key)?;
            }

            txn.delete(Db::Metadata, &metadata_key)
        })
    }

    fn label_exists(txn: &dyn ReadTxn, label: &str) -> Result<bool> {
        let prefix = NodeLabelIndex::prefix(label);
        if let Some(entry) = txn.iter(Db::Labels, Scan::Prefix(&prefix))?.next() {
            entry?;
            return Ok(true);
        }
        Ok(false)
    }

    fn backfill(txn: &mut dyn WriteTxn, label: &str, property: &str) -> Result<()> {
        let prefix = NodeLabelIndex::prefix(label);
        let entries = txn.scan(Db::Labels, Scan::Prefix(&prefix), None)?;
        let nodes: Vec<_> = entries
            .iter()
            .filter_map(|e| NodeLabelIndex::decode_node_id(e.key))
            .collect();

        for node_id in nodes {
            let Some(node_bytes) = txn.get(Db::Nodes, &node_id.to_be_bytes())? else {
                continue;
            };

            let node: Node = bincode::deserialize(&node_bytes)
                .map_err(|e| HelixiteError::Codec(e.to_string()))?;

            if node.label != label {
                continue;
            }

            let Some(value) = node.properties.get(property) else {
                continue;
            };

            let Some(prop_key) = NodePropertyIndex::key(label, property, value, node_id) else {
                continue;
            };

            txn.put(Db::Properties, &prop_key, &[])?;
        }

        Ok(())
    }

    pub(crate) fn delete(
        txn: &mut dyn WriteTxn,
        registry: &PropertyIndexRegistry,
        node: &Node,
    ) -> Result<()> {
        for (prop_name, value) in &node.properties {
            if registry.contains(&node.label, prop_name)
                && let Some(key) = NodePropertyIndex::key(&node.label, prop_name, value, node.id)
            {
                txn.delete(Db::Properties, &key)?;
            }
        }
        Ok(())
    }
}

pub(crate) struct EdgePropertyIndexes;

impl EdgePropertyIndexes {
    pub(crate) fn create(storage: &impl StorageEngine, label: &str, property: &str) -> Result<()> {
        let metadata_key = PropertyIndexMetadata::edge_key(label, property);

        storage.write(|txn| {
            if !Self::label_exists(txn, label)? {
                return Err(HelixiteError::LabelNotFound(label.to_string()));
            }

            if txn.get(Db::Metadata, &metadata_key)?.is_some() {
                return Err(HelixiteError::DuplicateKey(format!(
                    "edge property index {label}::{property} already exists"
                )));
            }

            Self::backfill(txn, label, property)?;
            txn.put(Db::Metadata, &metadata_key, &[])
        })
    }

    pub(crate) fn drop(storage: &impl StorageEngine, label: &str, property: &str) -> Result<()> {
        let metadata_key = PropertyIndexMetadata::edge_key(label, property);

        storage.write(|txn| {
            if txn.get(Db::Metadata, &metadata_key)?.is_none() {
                return Err(HelixiteError::IndexNotFound(format!(
                    "edge property index {label}::{property}"
                )));
            }

            let prefix = EdgePropertyIndex::index_prefix(label, property);
            let entries = txn.scan(Db::Properties, Scan::Prefix(&prefix), None)?;
            let keys: Vec<Vec<u8>> = entries.iter().map(|e| e.key.to_vec()).collect();
            for key in keys {
                txn.delete(Db::Properties, &key)?;
            }

            txn.delete(Db::Metadata, &metadata_key)
        })
    }

    pub(crate) fn insert(
        txn: &mut dyn WriteTxn,
        registry: &PropertyIndexRegistry,
        edge: &Edge,
    ) -> Result<()> {
        for (prop_name, value) in &edge.properties {
            if registry.contains(&edge.label, prop_name)
                && let Some(key) = EdgePropertyIndex::key(&edge.label, prop_name, value, edge.id)
            {
                txn.put(Db::Properties, &key, &[])?;
            }
        }
        Ok(())
    }

    pub(crate) fn replace(
        txn: &mut dyn WriteTxn,
        registry: &PropertyIndexRegistry,
        old: &Edge,
        new: &Edge,
    ) -> Result<()> {
        if old.label != new.label {
            for (prop_name, value) in &old.properties {
                if registry.contains(&old.label, prop_name)
                    && let Some(key) = EdgePropertyIndex::key(&old.label, prop_name, value, old.id)
                {
                    txn.delete(Db::Properties, &key)?;
                }
            }

            for (prop_name, value) in &new.properties {
                if registry.contains(&new.label, prop_name)
                    && let Some(key) = EdgePropertyIndex::key(&new.label, prop_name, value, new.id)
                {
                    txn.put(Db::Properties, &key, &[])?;
                }
            }
        } else {
            for (prop_name, old_value) in &old.properties {
                let new_value = new.properties.get(prop_name);
                if new_value == Some(old_value) {
                    continue;
                }

                if registry.contains(&old.label, prop_name)
                    && let Some(key) =
                        EdgePropertyIndex::key(&old.label, prop_name, old_value, old.id)
                {
                    txn.delete(Db::Properties, &key)?;
                }
            }

            for (prop_name, new_value) in &new.properties {
                let old_value = old.properties.get(prop_name);
                if old_value == Some(new_value) {
                    continue;
                }

                if registry.contains(&new.label, prop_name)
                    && let Some(key) =
                        EdgePropertyIndex::key(&new.label, prop_name, new_value, new.id)
                {
                    txn.put(Db::Properties, &key, &[])?;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn delete(
        txn: &mut dyn WriteTxn,
        registry: &PropertyIndexRegistry,
        edge: &Edge,
    ) -> Result<()> {
        for (prop_name, value) in &edge.properties {
            if registry.contains(&edge.label, prop_name)
                && let Some(key) = EdgePropertyIndex::key(&edge.label, prop_name, value, edge.id)
            {
                txn.delete(Db::Properties, &key)?;
            }
        }
        Ok(())
    }

    fn label_exists(txn: &dyn ReadTxn, label: &str) -> Result<bool> {
        let entries = txn.scan(Db::Edges, Scan::All, None)?;
        for entry in &entries {
            if let Ok(edge) = bincode::deserialize::<Edge>(entry.value)
                && edge.label == label
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn backfill(txn: &mut dyn WriteTxn, label: &str, property: &str) -> Result<()> {
        let entries = txn.scan(Db::Edges, Scan::All, None)?;
        let edges: Vec<_> = entries
            .iter()
            .filter_map(|e| bincode::deserialize::<Edge>(e.value).ok())
            .collect();

        for edge in edges {
            if edge.label != label {
                continue;
            }

            let Some(prop_value) = edge.properties.get(property) else {
                continue;
            };

            let Some(prop_key) = EdgePropertyIndex::key(label, property, prop_value, edge.id)
            else {
                continue;
            };

            txn.put(Db::Properties, &prop_key, &[])?;
        }

        Ok(())
    }
}
