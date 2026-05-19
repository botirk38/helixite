use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::storage::StorageTxn;
use crate::storage::engine::Db;

const METADATA_NEXT_NODE_ID: &[u8] = b"next_node_id";
const METADATA_NEXT_EDGE_ID: &[u8] = b"next_edge_id";

pub(crate) struct IdAllocator;

impl IdAllocator {
    pub(crate) fn next_node_id(txn: &mut dyn StorageTxn) -> Result<NodeId> {
        Self::next_id(txn, METADATA_NEXT_NODE_ID, "next_node_id")
    }

    pub(crate) fn next_edge_id(txn: &mut dyn StorageTxn) -> Result<EdgeId> {
        Self::next_id(txn, METADATA_NEXT_EDGE_ID, "next_edge_id")
    }

    fn next_id(txn: &mut dyn StorageTxn, key: &[u8], name: &str) -> Result<u64> {
        let bytes = match txn.get(Db::Metadata, key)? {
            Some(b) => b,
            None => {
                txn.put(Db::Metadata, key, &2u64.to_be_bytes())?;
                return Ok(1);
            }
        };

        let bytes: [u8; 8] = bytes
            .try_into()
            .map_err(|_| HelixiteError::Storage(format!("invalid {name} metadata value")))?;

        let next_id = u64::from_be_bytes(bytes);

        let following_id = next_id
            .checked_add(1)
            .ok_or_else(|| HelixiteError::Storage(format!("{name} overflow")))?;

        txn.put(Db::Metadata, key, &following_id.to_be_bytes())?;

        Ok(next_id)
    }
}
