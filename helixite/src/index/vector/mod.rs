mod hnsw;
mod keys;
mod similarity;

use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::storage::{ReadTxn, WriteTxn};
use crate::value::Value;

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
        storage.write(|txn| {
            if txn
                .get(Db::VectorIndexes, &keys::meta_key(label, property))?
                .is_some()
            {
                return Err(HelixiteError::DuplicateKey(format!(
                    "vector index {label}::{property} already exists"
                )));
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
            txn.put(
                Db::VectorIndexes,
                &keys::meta_key(label, property),
                &meta.serialize(),
            )?;

            Self::backfill(txn, label, property)?;

            Ok(())
        })
    }

    fn backfill(txn: &mut dyn WriteTxn, label: &str, property: &str) -> Result<()> {
        let prefix = crate::index::labels::LabelIndex::prefix(label);
        let entries = txn.scan_prefix(Db::Labels, &prefix)?;

        for (key, _) in entries {
            let Some(node_id) = crate::index::labels::LabelIndex::decode_node_id(&key) else {
                continue;
            };

            let Some(node_bytes) = txn.get(Db::Nodes, &node_id.to_be_bytes())? else {
                continue;
            };

            let node: Node = bincode::deserialize(&node_bytes)
                .map_err(|e| HelixiteError::Codec(e.to_string()))?;

            if node.label != label {
                continue;
            }

            let Some(value) = node.properties.get(property) else {
                continue;
            };

            let Value::Vector(vector) = value else {
                continue;
            };

            let meta = Self::load_meta(txn, label, property)?;
            Self::insert(txn, label, property, node_id, vector, &meta)?;
        }

        Ok(())
    }

    pub(crate) fn drop(storage: &impl StorageEngine, label: &str, property: &str) -> Result<()> {
        storage.write(|txn| {
            if txn
                .get(Db::VectorIndexes, &keys::meta_key(label, property))?
                .is_none()
            {
                return Err(HelixiteError::IndexNotFound(format!(
                    "vector index {label}::{property}"
                )));
            }

            let vec_prefix = keys::vec_prefix(label, property);
            let vec_entries = txn.scan_prefix(Db::VectorIndexes, &vec_prefix)?;
            for (key, _) in vec_entries {
                txn.delete(Db::VectorIndexes, &key)?;
            }

            let lvl_prefix = keys::lvl_prefix(label, property);
            let lvl_entries = txn.scan_prefix(Db::VectorIndexes, &lvl_prefix)?;
            for (key, _) in lvl_entries {
                txn.delete(Db::VectorIndexes, &key)?;
            }

            let lnk_prefix = keys::lnk_index_prefix(label, property);
            let lnk_entries = txn.scan_prefix(Db::VectorIndexes, &lnk_prefix)?;
            for (key, _) in lnk_entries {
                txn.delete(Db::VectorIndexes, &key)?;
            }

            txn.delete(Db::VectorIndexes, &keys::meta_key(label, property))?;
            txn.delete(Db::VectorIndexes, &keys::ep_key(label, property))?;

            Ok(())
        })
    }

    pub(crate) fn load_meta(
        txn: &dyn ReadTxn,
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

    pub(crate) fn insert(
        txn: &mut dyn WriteTxn,
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
        Hnsw::insert(txn, label, property, node_id, vector, meta)
    }

    pub(crate) fn delete(
        txn: &mut dyn WriteTxn,
        label: &str,
        property: &str,
        node_id: NodeId,
        meta: &VectorIndexMeta,
    ) -> Result<()> {
        Hnsw::delete(txn, label, property, node_id, meta)
    }

    pub(crate) fn search(
        txn: &dyn ReadTxn,
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
        Hnsw::search(txn, label, property, query, k, meta)
    }
}
