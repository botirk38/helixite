use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::query::{NodeQuery, NodeRefQuery};
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::storage::lmdb::LmdbStorage;
use crate::value::Value;

use crate::index::edges::EdgeIndex;
use crate::index::labels::LabelIndex;
use crate::index::properties::PropertyIndex;
use crate::index::vector::{HnswConfig, VectorIndex};

const METADATA_NEXT_NODE_ID: &[u8] = b"next_node_id";
const METADATA_NEXT_EDGE_ID: &[u8] = b"next_edge_id";

pub struct Helixite<S: StorageEngine = LmdbStorage> {
    path: PathBuf,
    storage: S,
}

pub struct HelixiteBuilder {
    config: Config,
}

#[expect(clippy::derivable_impls)]
impl Default for HelixiteBuilder {
    fn default() -> Self {
        Self {
            config: Config::default(),
        }
    }
}

pub struct HelixiteStorageBuilder<S: StorageEngine> {
    storage: S,
}

impl HelixiteBuilder {
    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn storage<S: StorageEngine>(self, storage: S) -> HelixiteStorageBuilder<S> {
        HelixiteStorageBuilder { storage }
    }

    pub fn open(self, path: impl AsRef<Path>) -> Result<Helixite> {
        let path = path.as_ref().to_path_buf();
        let storage = LmdbStorage::open(&path, &self.config)?;
        Ok(Helixite { path, storage })
    }
}

impl<S: StorageEngine> HelixiteStorageBuilder<S> {
    pub fn open(self, path: impl AsRef<Path>) -> Result<Helixite<S>> {
        Ok(Helixite {
            path: path.as_ref().to_path_buf(),
            storage: self.storage,
        })
    }
}

impl<S: StorageEngine> Helixite<S> {
    pub fn add_node(
        &self,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<NodeId> {
        let label = label.into();
        let properties: std::collections::BTreeMap<String, Value> =
            properties.into_iter().collect();

        for (prop_name, prop_value) in &properties {
            if let Value::Vector(vector) = prop_value
                && let Ok(meta) = VectorIndex::load_meta(&self.storage, &label, prop_name)
                && vector.len() != meta.dimension
            {
                return Err(HelixiteError::InvalidVectorDim {
                    expected: meta.dimension,
                    actual: vector.len(),
                });
            }
        }

        let node_id = self.storage.write(|txn| {
            let next_id = match txn.get(Db::Metadata, METADATA_NEXT_NODE_ID)? {
                Some(bytes) => decode_u64(&bytes, "next_node_id")?,
                None => 1,
            };

            let node = Node {
                id: next_id,
                label: label.clone(),
                properties: properties.clone(),
            };

            let bytes =
                bincode::serialize(&node).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Nodes, &next_id.to_be_bytes(), &bytes)?;

            let label_k = LabelIndex::key(&label, next_id);
            txn.put(Db::Labels, &label_k, &[])?;

            for (prop_name, prop_value) in &node.properties {
                if let Some(key) = PropertyIndex::key(prop_name, prop_value, next_id) {
                    txn.put(Db::Properties, &key, &[])?;
                }
                if let Value::Vector(vector) = prop_value
                    && let Ok(meta) = VectorIndex::load_meta_from_txn(txn, &label, prop_name)
                {
                    VectorIndex::insert_into_txn(txn, &label, prop_name, next_id, vector, &meta)?;
                }
            }

            txn.put(
                Db::Metadata,
                METADATA_NEXT_NODE_ID,
                &(next_id + 1).to_be_bytes(),
            )?;

            Ok(next_id)
        })?;

        Ok(node_id)
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .storage
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        let node = bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(node)
    }

    pub fn update_node(
        &self,
        id: NodeId,
        label: Option<impl Into<String>>,
        properties: Option<impl IntoIterator<Item = (String, Value)>>,
    ) -> Result<()> {
        let current = self.get_node(id)?;

        let new_label = label.map(Into::into);
        let new_properties =
            properties.map(|p| p.into_iter().collect::<std::collections::BTreeMap<_, _>>());

        for (prop_name, prop_value) in new_properties.iter().flatten() {
            if let Value::Vector(vector) = prop_value {
                let label_ref = new_label.as_ref().unwrap_or(&current.label);
                if let Ok(meta) = VectorIndex::load_meta(&self.storage, label_ref, prop_name)
                    && vector.len() != meta.dimension
                {
                    return Err(HelixiteError::InvalidVectorDim {
                        expected: meta.dimension,
                        actual: vector.len(),
                    });
                }
            }
        }

        self.storage.write(|txn| {
            let mut updated = current.clone();

            if let Some(ref new_label) = new_label {
                let old_label_key = LabelIndex::key(&current.label, id);
                txn.delete(Db::Labels, &old_label_key)?;
                let new_label_key = LabelIndex::new_label_key(new_label, id);
                txn.put(Db::Labels, &new_label_key, &[])?;
                updated.label = new_label.clone();
            }

            if let Some(ref new_props) = new_properties {
                for (prop_name, old_value) in &current.properties {
                    if let Some(key) = PropertyIndex::key(prop_name, old_value, id) {
                        txn.delete(Db::Properties, &key)?;
                    }
                    if let Value::Vector(_) = old_value
                        && let Ok(meta) =
                            VectorIndex::load_meta_from_txn(txn, &updated.label, prop_name)
                    {
                        VectorIndex::delete_from_txn(txn, &updated.label, prop_name, id, &meta)?;
                    }
                }

                for (prop_name, new_value) in new_props {
                    if let Some(key) = PropertyIndex::key(prop_name, new_value, id) {
                        txn.put(Db::Properties, &key, &[])?;
                    }
                    if let Value::Vector(vector) = new_value
                        && let Ok(meta) =
                            VectorIndex::load_meta_from_txn(txn, &updated.label, prop_name)
                    {
                        VectorIndex::insert_into_txn(
                            txn,
                            &updated.label,
                            prop_name,
                            id,
                            vector,
                            &meta,
                        )?;
                    }
                }

                updated.properties = new_props.clone();
            }

            let bytes =
                bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Nodes, &id.to_be_bytes(), &bytes)?;

            Ok(())
        })
    }

    pub fn add_edge(
        &self,
        from: NodeId,
        to: NodeId,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<EdgeId> {
        let label = label.into();
        self.storage.write(|txn| {
            let next_id = match txn.get(Db::Metadata, METADATA_NEXT_EDGE_ID)? {
                Some(bytes) => decode_u64(&bytes, "next_edge_id")?,
                None => 1,
            };

            let edge = Edge {
                id: next_id,
                from,
                to,
                label: label.clone(),
                properties: properties.into_iter().collect(),
            };

            let bytes =
                bincode::serialize(&edge).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Edges, &next_id.to_be_bytes(), &bytes)?;

            let out_k = EdgeIndex::out_key(from, &label, next_id);
            txn.put(Db::OutEdges, &out_k, &next_id.to_be_bytes())?;

            let in_k = EdgeIndex::in_key(to, &label, next_id);
            txn.put(Db::InEdges, &in_k, &next_id.to_be_bytes())?;

            txn.put(
                Db::Metadata,
                METADATA_NEXT_EDGE_ID,
                &(next_id + 1).to_be_bytes(),
            )?;

            Ok(next_id)
        })
    }

    pub fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        let bytes = self
            .storage
            .get(Db::Edges, &id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(id))?;

        let edge = bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(edge)
    }

    pub fn update_edge(
        &self,
        id: EdgeId,
        label: Option<impl Into<String>>,
        properties: Option<impl IntoIterator<Item = (String, Value)>>,
    ) -> Result<()> {
        let current = self.get_edge(id)?;

        let new_label = label.map(Into::into);
        let new_properties =
            properties.map(|p| p.into_iter().collect::<std::collections::BTreeMap<_, _>>());

        self.storage.write(|txn| {
            let mut updated = current.clone();

            if let Some(ref new_label) = new_label {
                let old_out = EdgeIndex::out_key(current.from, &current.label, id);
                txn.delete(Db::OutEdges, &old_out)?;
                let old_in = EdgeIndex::in_key(current.to, &current.label, id);
                txn.delete(Db::InEdges, &old_in)?;

                let new_out = EdgeIndex::out_key(current.from, new_label, id);
                txn.put(Db::OutEdges, &new_out, &id.to_be_bytes())?;
                let new_in = EdgeIndex::in_key(current.to, new_label, id);
                txn.put(Db::InEdges, &new_in, &id.to_be_bytes())?;
                updated.label = new_label.clone();
            }

            if let Some(ref new_props) = new_properties {
                updated.properties = new_props.clone();
            }

            let bytes =
                bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Edges, &id.to_be_bytes(), &bytes)?;

            Ok(())
        })
    }

    pub fn nodes(&self) -> NodeQuery<'_, S> {
        NodeQuery::new(&self.storage)
    }

    pub fn node(&self, id: NodeId) -> NodeRefQuery<'_, S> {
        NodeRefQuery::new(&self.storage, id)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn create_vector_index(
        &self,
        label: &str,
        property: &str,
        dimension: usize,
        config: HnswConfig,
    ) -> Result<()> {
        VectorIndex::create(&self.storage, label, property, dimension, config)
    }
}

fn decode_u64(bytes: &[u8], name: &str) -> Result<u64> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| HelixiteError::Storage(format!("invalid {name} metadata value")))?;
    Ok(u64::from_be_bytes(bytes))
}
