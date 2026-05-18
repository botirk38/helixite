use crate::edge::Direction;
use crate::id::{EdgeId, NodeId};
use crate::storage::StorageEngine;
use crate::storage::engine::Db;

use super::codec::{KeyBuilder, KeyReader};

pub(crate) struct EdgeIndex;

impl EdgeIndex {
    pub(crate) fn out_key(node_id: NodeId, label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new()
            .u64(node_id)
            .str(label)
            .u64(edge_id)
            .finish()
    }

    pub(crate) fn out_prefix(node_id: NodeId, label: Option<&str>) -> Vec<u8> {
        let mut builder = KeyBuilder::new().u64(node_id);
        if let Some(l) = label {
            builder = builder.str(l);
        }
        builder.finish()
    }

    pub(crate) fn in_key(node_id: NodeId, label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new()
            .u64(node_id)
            .str(label)
            .u64(edge_id)
            .finish()
    }

    pub(crate) fn in_prefix(node_id: NodeId, label: Option<&str>) -> Vec<u8> {
        let mut builder = KeyBuilder::new().u64(node_id);
        if let Some(l) = label {
            builder = builder.str(l);
        }
        builder.finish()
    }

    pub(crate) fn decode_edge_id(key: &[u8]) -> Option<EdgeId> {
        let mut reader = KeyReader::new(key);
        reader.u64()?;
        reader.str()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }

    pub(crate) fn decode_target_node<S: StorageEngine>(
        storage: &S,
        key: &[u8],
        direction: Direction,
    ) -> Option<NodeId> {
        let edge_id = Self::decode_edge_id(key)?;
        let edge_bytes = storage.get(Db::Edges, &edge_id.to_be_bytes()).ok()??;
        let edge: crate::edge::Edge = bincode::deserialize(&edge_bytes).ok()?;
        match direction {
            Direction::Out => Some(edge.to),
            Direction::In => Some(edge.from),
        }
    }
}
