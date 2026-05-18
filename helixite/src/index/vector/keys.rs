use crate::error::Result;
use crate::id::NodeId;

use super::super::codec::{KeyBuilder, KeyReader};
use super::similarity::SimilarityKind;

const META_PREFIX: u8 = 0;
const VEC_PREFIX: u8 = 1;
const LVL_PREFIX: u8 = 2;
const LNK_PREFIX: u8 = 3;
const EP_PREFIX: u8 = 4;

pub(crate) fn meta_key(label: &str, property: &str) -> Vec<u8> {
    KeyBuilder::new()
        .u8(META_PREFIX)
        .str(label)
        .str(property)
        .finish()
}

pub(crate) fn vec_key(label: &str, property: &str, node_id: NodeId) -> Vec<u8> {
    KeyBuilder::new()
        .u8(VEC_PREFIX)
        .str(label)
        .str(property)
        .u64(node_id)
        .finish()
}

pub(crate) fn lvl_key(label: &str, property: &str, node_id: NodeId) -> Vec<u8> {
    KeyBuilder::new()
        .u8(LVL_PREFIX)
        .str(label)
        .str(property)
        .u64(node_id)
        .finish()
}

pub(crate) fn lnk_key(
    label: &str,
    property: &str,
    level: u8,
    node_id: NodeId,
    neighbor_id: NodeId,
) -> Vec<u8> {
    KeyBuilder::new()
        .u8(LNK_PREFIX)
        .str(label)
        .str(property)
        .u8(level)
        .u64(node_id)
        .u64(neighbor_id)
        .finish()
}

pub(crate) fn lnk_prefix(label: &str, property: &str, level: u8, node_id: NodeId) -> Vec<u8> {
    KeyBuilder::new()
        .u8(LNK_PREFIX)
        .str(label)
        .str(property)
        .u8(level)
        .u64(node_id)
        .finish()
}

pub(crate) fn ep_key(label: &str, property: &str) -> Vec<u8> {
    KeyBuilder::new()
        .u8(EP_PREFIX)
        .str(label)
        .str(property)
        .finish()
}

pub(crate) fn decode_link_from_lnk_key(key: &[u8]) -> Option<(u8, NodeId, NodeId)> {
    let mut reader = KeyReader::new(key);
    reader.u8()?;
    reader.str()?;
    reader.str()?;
    let level = reader.u8()?;
    let node_id = reader.u64()?;
    let neighbor_id = reader.u64()?;
    reader.finish()?;
    Some((level, node_id, neighbor_id))
}

#[derive(Debug, Clone)]
pub(crate) struct VectorIndexMeta {
    pub(crate) dimension: usize,
    pub(crate) m: usize,
    pub(crate) ef_construction: usize,
    pub(crate) ef_search: usize,
    pub(crate) similarity: SimilarityKind,
    pub(crate) entry_point: Option<u64>,
    pub(crate) max_level: u8,
}

impl VectorIndexMeta {
    pub(crate) fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(33);
        bytes.extend((self.dimension as u32).to_le_bytes());
        bytes.extend((self.m as u32).to_le_bytes());
        bytes.extend((self.ef_construction as u32).to_le_bytes());
        bytes.extend((self.ef_search as u32).to_le_bytes());
        bytes.push(self.similarity.to_byte());
        bytes.extend(self.entry_point.unwrap_or(0).to_le_bytes());
        bytes.push(self.max_level);
        bytes
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 26 {
            return Err(crate::error::HelixiteError::Codec(
                "vector index metadata too short".into(),
            ));
        }
        let dimension = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let m = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let ef_construction = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let ef_search = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let similarity = SimilarityKind::from_byte(bytes[16])?;
        let entry_point_raw = u64::from_le_bytes(bytes[17..25].try_into().unwrap());
        let entry_point = if entry_point_raw == 0 {
            None
        } else {
            Some(entry_point_raw)
        };
        let max_level = bytes[25];
        Ok(Self {
            dimension,
            m,
            ef_construction,
            ef_search,
            similarity,
            entry_point,
            max_level,
        })
    }
}

pub(crate) fn serialize_vector(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for &v in vector {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

pub(crate) fn deserialize_vector(bytes: &[u8]) -> Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(4) {
        return Err(crate::error::HelixiteError::Codec(
            "corrupt vector data: length not multiple of 4".into(),
        ));
    }
    let mut vector = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let f = f32::from_le_bytes(chunk.try_into().unwrap());
        vector.push(f);
    }
    Ok(vector)
}
