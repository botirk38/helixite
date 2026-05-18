mod exact;
mod hnsw;

use crate::id::NodeId;
use crate::storage::StorageEngine;

use super::codec::KeyBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorIndexKind {
    Exact,
    Hnsw,
}

pub struct VectorIndex;

impl VectorIndex {
    pub fn key(kind: VectorIndexKind, label: &str, property: &str, node_id: NodeId) -> Vec<u8> {
        KeyBuilder::new()
            .u8(kind as u8)
            .str(label)
            .str(property)
            .u64(node_id)
            .finish()
    }

    pub fn prefix(kind: VectorIndexKind, label: &str, property: &str) -> Vec<u8> {
        KeyBuilder::new()
            .u8(kind as u8)
            .str(label)
            .str(property)
            .finish()
    }

    pub fn decode_node_id(key: &[u8]) -> Option<NodeId> {
        let mut reader = super::codec::KeyReader::new(key);
        reader.u8()?;
        reader.str()?;
        reader.str()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }

    pub fn serialize_vector(vector: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vector.len() * 4);
        for &v in vector {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }

    pub fn search(
        storage: &impl StorageEngine,
        kind: VectorIndexKind,
        label: &str,
        property: &str,
        query: &[f32],
        k: usize,
    ) -> crate::error::Result<Vec<(NodeId, f32)>> {
        match kind {
            VectorIndexKind::Exact => exact::search(storage, label, property, query, k),
            VectorIndexKind::Hnsw => hnsw::search(storage, label, property, query, k),
        }
    }
}
