use crate::id::NodeId;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;

use super::VectorIndex;

pub(crate) fn search(
    storage: &impl StorageEngine,
    label: &str,
    property: &str,
    query: &[f32],
    k: usize,
) -> crate::error::Result<Vec<(NodeId, f32)>> {
    let prefix = VectorIndex::prefix(super::VectorIndexKind::Exact, label, property);
    let entries = storage.scan_prefix(Db::VectorIndexes, &prefix)?;

    let mut results = Vec::new();
    for (key, value) in entries {
        let node_id = VectorIndex::decode_node_id(&key).ok_or_else(|| {
            crate::error::HelixiteError::Storage("corrupt vector index key".into())
        })?;
        let vector = deserialize_vector(&value)?;
        let similarity = cosine_similarity(&vector, query);
        results.push((node_id, similarity));
    }

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(k);
    Ok(results)
}

fn deserialize_vector(bytes: &[u8]) -> crate::error::Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(4) {
        return Err(crate::error::HelixiteError::Storage(
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

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}
