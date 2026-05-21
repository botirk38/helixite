use crate::id::{EdgeId, NodeId};

use super::codec::{KeyBuilder, KeyReader};

pub(crate) struct NodeLabelIndex;

impl NodeLabelIndex {
    pub(crate) fn key(label: &str, node_id: NodeId) -> Vec<u8> {
        KeyBuilder::new().str(label).u64(node_id).finish()
    }

    pub(crate) fn prefix(label: &str) -> Vec<u8> {
        KeyBuilder::new().str(label).finish()
    }

    pub(crate) fn decode_node_id(key: &[u8]) -> Option<NodeId> {
        let mut reader = KeyReader::new(key);
        reader.str()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }
}

pub(crate) struct EdgeLabelIndex;

impl EdgeLabelIndex {
    pub(crate) fn key(label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new().u8(1).str(label).u64(edge_id).finish()
    }

    pub(crate) fn prefix(label: &str) -> Vec<u8> {
        KeyBuilder::new().u8(1).str(label).finish()
    }

    pub(crate) fn decode_edge_id(key: &[u8]) -> Option<EdgeId> {
        let mut reader = KeyReader::new(key);
        reader.u8()?;
        reader.str()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }
}
