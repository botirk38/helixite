use crate::edge::Direction;
use crate::id::{EdgeId, NodeId};

use super::codec::{KeyBuilder, KeyReader};

pub struct EdgeIndex;

pub struct DecodedOutEdge {
    pub from: NodeId,
    pub label: String,
    pub edge_id: EdgeId,
}

pub struct DecodedInEdge {
    pub to: NodeId,
    pub label: String,
    pub edge_id: EdgeId,
}

impl EdgeIndex {
    pub fn out_key(node_id: NodeId, label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new()
            .u64(node_id)
            .str(label)
            .u64(edge_id)
            .finish()
    }

    pub fn out_prefix(node_id: NodeId, label: Option<&str>) -> Vec<u8> {
        let mut builder = KeyBuilder::new().u64(node_id);
        if let Some(l) = label {
            builder = builder.str(l);
        }
        builder.finish()
    }

    pub fn in_key(node_id: NodeId, label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new()
            .u64(node_id)
            .str(label)
            .u64(edge_id)
            .finish()
    }

    pub fn in_prefix(node_id: NodeId, label: Option<&str>) -> Vec<u8> {
        let mut builder = KeyBuilder::new().u64(node_id);
        if let Some(l) = label {
            builder = builder.str(l);
        }
        builder.finish()
    }

    pub fn decode_out_edge(key: &[u8]) -> Option<DecodedOutEdge> {
        let mut reader = KeyReader::new(key);
        let from = reader.u64()?;
        let label = std::str::from_utf8(reader.str()?).ok()?;
        let edge_id = reader.u64()?;
        reader.finish()?;
        Some(DecodedOutEdge {
            from,
            label: label.to_string(),
            edge_id,
        })
    }

    pub fn decode_in_edge(key: &[u8]) -> Option<DecodedInEdge> {
        let mut reader = KeyReader::new(key);
        let to = reader.u64()?;
        let label = std::str::from_utf8(reader.str()?).ok()?;
        let edge_id = reader.u64()?;
        reader.finish()?;
        Some(DecodedInEdge {
            to,
            label: label.to_string(),
            edge_id,
        })
    }

    pub fn decode_edge_id(key: &[u8]) -> Option<EdgeId> {
        let mut reader = KeyReader::new(key);
        reader.u64()?;
        reader.str()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }

    pub fn decode_target_from_edge(edge: &crate::edge::Edge, direction: Direction) -> NodeId {
        match direction {
            Direction::Out => edge.to,
            Direction::In => edge.from,
        }
    }
}
