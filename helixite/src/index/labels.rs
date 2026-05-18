use crate::id::NodeId;

use super::codec::{KeyBuilder, KeyReader};

pub(crate) struct LabelIndex;

impl LabelIndex {
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

    pub(crate) fn new_label_key(new_label: &str, node_id: NodeId) -> Vec<u8> {
        Self::key(new_label, node_id)
    }
}
