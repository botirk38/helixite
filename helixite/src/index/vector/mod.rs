mod hnsw;
mod keys;
mod similarity;

use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;

pub use similarity::SimilarityFn;
pub use similarity::SimilarityKind;

pub(crate) use hnsw::Hnsw;
pub(crate) use keys::VectorIndexMeta;

#[derive(Debug, Clone)]
pub struct HnswConfig {
    pub(crate) m: usize,
    pub(crate) ef_construction: usize,
    pub(crate) ef_search: usize,
    pub(crate) similarity: SimilarityKind,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            similarity: SimilarityKind::Cosine,
        }
    }
}

impl HnswConfig {
    pub fn cosine() -> Self {
        Self {
            similarity: SimilarityKind::Cosine,
            ..Default::default()
        }
    }

    pub fn dot_product() -> Self {
        Self {
            similarity: SimilarityKind::DotProduct,
            ..Default::default()
        }
    }

    pub fn euclidean() -> Self {
        Self {
            similarity: SimilarityKind::Euclidean,
            ..Default::default()
        }
    }

    pub fn custom(f: SimilarityFn) -> Self {
        Self {
            similarity: SimilarityKind::Custom(f),
            ..Default::default()
        }
    }

    pub fn m(mut self, m: usize) -> Self {
        self.m = m;
        self
    }

    pub fn ef_construction(mut self, ef: usize) -> Self {
        self.ef_construction = ef;
        self
    }

    pub fn ef_search(mut self, ef: usize) -> Self {
        self.ef_search = ef;
        self
    }

    pub fn similarity(mut self, s: SimilarityKind) -> Self {
        self.similarity = s;
        self
    }
}

pub(crate) struct VectorIndex;

impl VectorIndex {
    pub(crate) fn create(
        storage: &impl StorageEngine,
        label: &str,
        property: &str,
        dimension: usize,
        config: HnswConfig,
    ) -> Result<()> {
        if matches!(config.similarity, SimilarityKind::Custom(_)) {
            return Err(HelixiteError::Codec(
                "custom similarity cannot be used with persisted indexes".into(),
            ));
        }
        let meta = VectorIndexMeta {
            dimension,
            m: config.m,
            ef_construction: config.ef_construction,
            ef_search: config.ef_search,
            similarity: config.similarity,
            entry_point: None,
            max_level: 0,
        };
        storage.write(|txn| {
            txn.put(
                Db::VectorIndexes,
                &keys::meta_key(label, property),
                &meta.serialize(),
            )?;
            Ok(())
        })
    }

    pub(crate) fn load_meta(
        storage: &impl StorageEngine,
        label: &str,
        property: &str,
    ) -> Result<VectorIndexMeta> {
        let bytes = storage
            .get(Db::VectorIndexes, &keys::meta_key(label, property))?
            .ok_or_else(|| HelixiteError::VectorIndexNotFound {
                label: label.into(),
                property: property.into(),
            })?;
        VectorIndexMeta::deserialize(&bytes)
    }

    pub(crate) fn load_meta_from_txn(
        txn: &mut dyn crate::storage::StorageTxn,
        label: &str,
        property: &str,
    ) -> Result<VectorIndexMeta> {
        let bytes = txn
            .get(Db::VectorIndexes, &keys::meta_key(label, property))?
            .ok_or_else(|| HelixiteError::VectorIndexNotFound {
                label: label.into(),
                property: property.into(),
            })?;
        VectorIndexMeta::deserialize(&bytes)
    }

    pub(crate) fn insert_into_txn(
        txn: &mut dyn crate::storage::StorageTxn,
        label: &str,
        property: &str,
        node_id: NodeId,
        vector: &[f32],
        meta: &VectorIndexMeta,
    ) -> Result<()> {
        if vector.len() != meta.dimension {
            return Err(HelixiteError::InvalidVectorDim {
                expected: meta.dimension,
                actual: vector.len(),
            });
        }
        Hnsw::insert_into_txn(txn, label, property, node_id, vector, meta)
    }

    pub(crate) fn search(
        storage: &impl StorageEngine,
        label: &str,
        property: &str,
        query: &[f32],
        k: usize,
        meta: &VectorIndexMeta,
    ) -> Result<Vec<(NodeId, f32)>> {
        if query.len() != meta.dimension {
            return Err(HelixiteError::InvalidVectorDim {
                expected: meta.dimension,
                actual: query.len(),
            });
        }
        Hnsw::search(storage, label, property, query, k, meta)
    }
}
