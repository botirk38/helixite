use crate::edge::Direction;
use crate::error::Result;
use crate::id::{EdgeId, NodeId};
use crate::storage::StorageTxn;
use crate::storage::engine::Db;

use super::codec::{KeyBuilder, KeyReader};

pub(crate) struct EdgeIndex;

impl EdgeIndex {
    pub(crate) fn out_key(from: NodeId, label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new().u64(from).str(label).u64(edge_id).finish()
    }

    pub(crate) fn out_prefix(from: NodeId, label: Option<&str>) -> Vec<u8> {
        let mut builder = KeyBuilder::new().u64(from);
        if let Some(l) = label {
            builder = builder.str(l);
        }
        builder.finish()
    }

    pub(crate) fn in_key(to: NodeId, label: &str, edge_id: EdgeId) -> Vec<u8> {
        KeyBuilder::new().u64(to).str(label).u64(edge_id).finish()
    }

    pub(crate) fn in_prefix(to: NodeId, label: Option<&str>) -> Vec<u8> {
        let mut builder = KeyBuilder::new().u64(to);
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

    pub(crate) fn decode_target_from_edge(
        edge: &crate::edge::Edge,
        direction: Direction,
    ) -> NodeId {
        match direction {
            Direction::Out => edge.to,
            Direction::In => edge.from,
        }
    }

    pub(crate) fn insert(
        txn: &mut dyn StorageTxn,
        from: NodeId,
        to: NodeId,
        label: &str,
        edge_id: EdgeId,
    ) -> Result<()> {
        let out_k = Self::out_key(from, label, edge_id);
        txn.put(Db::OutEdges, &out_k, &edge_id.to_be_bytes())?;

        let in_k = Self::in_key(to, label, edge_id);
        txn.put(Db::InEdges, &in_k, &edge_id.to_be_bytes())?;

        Ok(())
    }

    pub(crate) fn replace_label(
        txn: &mut dyn StorageTxn,
        from: NodeId,
        to: NodeId,
        old_label: &str,
        new_label: &str,
        edge_id: EdgeId,
    ) -> Result<()> {
        let old_out = Self::out_key(from, old_label, edge_id);
        txn.delete(Db::OutEdges, &old_out)?;
        let old_in = Self::in_key(to, old_label, edge_id);
        txn.delete(Db::InEdges, &old_in)?;

        let new_out = Self::out_key(from, new_label, edge_id);
        txn.put(Db::OutEdges, &new_out, &edge_id.to_be_bytes())?;
        let new_in = Self::in_key(to, new_label, edge_id);
        txn.put(Db::InEdges, &new_in, &edge_id.to_be_bytes())?;

        Ok(())
    }

    pub(crate) fn delete(
        txn: &mut dyn StorageTxn,
        from: NodeId,
        to: NodeId,
        label: &str,
        edge_id: EdgeId,
    ) -> Result<()> {
        let out_k = Self::out_key(from, label, edge_id);
        txn.delete(Db::OutEdges, &out_k)?;

        let in_k = Self::in_key(to, label, edge_id);
        txn.delete(Db::InEdges, &in_k)?;

        Ok(())
    }
}
