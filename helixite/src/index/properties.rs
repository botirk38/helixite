use crate::id::NodeId;
use crate::value::Value;

use super::codec::{KeyBuilder, KeyReader};

pub struct PropertyIndex;

impl PropertyIndex {
    pub fn key(property: &str, value: &Value, node_id: NodeId) -> Option<Vec<u8>> {
        let value_bytes = value.to_index_key()?;
        Some(
            KeyBuilder::new()
                .str(property)
                .bytes(&value_bytes)
                .u64(node_id)
                .finish(),
        )
    }

    pub fn prefix(property: &str, value: &Value) -> Option<Vec<u8>> {
        let value_bytes = value.to_index_key()?;
        Some(KeyBuilder::new().str(property).bytes(&value_bytes).finish())
    }

    pub fn decode_node_id(key: &[u8]) -> Option<NodeId> {
        let mut reader = KeyReader::new(key);
        reader.str()?;
        reader.bytes()?;
        let id = reader.u64()?;
        reader.finish()?;
        Some(id)
    }
}
