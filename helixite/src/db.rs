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

pub struct NodeMutation<'a, S: StorageEngine> {
    db: &'a Helixite<S>,
    id: NodeId,
    ops: Vec<NodeOp>,
}

enum NodeOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(std::collections::BTreeMap<String, Value>),
}

impl<'a, S: StorageEngine> NodeMutation<'a, S> {
    pub fn set_label(mut self, label: impl Into<String>) -> Self {
        self.ops.push(NodeOp::SetLabel(label.into()));
        self
    }

    pub fn set_property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.ops.push(NodeOp::SetProperty(key.into(), value));
        self
    }

    pub fn remove_property(mut self, key: impl Into<String>) -> Self {
        self.ops.push(NodeOp::RemoveProperty(key.into()));
        self
    }

    pub fn replace_properties(
        mut self,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Self {
        let props: std::collections::BTreeMap<String, Value> = properties.into_iter().collect();
        self.ops.push(NodeOp::ReplaceProperties(props));
        self
    }

    pub fn apply(self) -> Result<()> {
        let current = self.db.get_node(self.id)?;

        let mut label = current.label.clone();
        let mut properties = current.properties.clone();

        for op in &self.ops {
            match op {
                NodeOp::SetLabel(l) => label = l.clone(),
                NodeOp::SetProperty(k, v) => {
                    properties.insert(k.clone(), v.clone());
                }
                NodeOp::RemoveProperty(k) => {
                    properties.remove(k);
                }
                NodeOp::ReplaceProperties(props) => {
                    properties = props.clone();
                }
            }
        }

        validate_vector_dims(&self.db.storage, &label, &properties)?;

        self.db.storage.write(|txn| {
            let old_label = &current.label;

            delete_node_indexes(
                txn,
                old_label,
                &label,
                self.id,
                &current.properties,
                &properties,
            )?;
            insert_node_indexes(txn, &label, self.id, &properties)?;

            let updated = Node {
                id: self.id,
                label: label.clone(),
                properties: properties.clone(),
            };

            let bytes =
                bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Nodes, &self.id.to_be_bytes(), &bytes)?;

            Ok(())
        })
    }
}

pub struct EdgeMutation<'a, S: StorageEngine> {
    db: &'a Helixite<S>,
    id: EdgeId,
    ops: Vec<EdgeOp>,
}

enum EdgeOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(std::collections::BTreeMap<String, Value>),
}

impl<'a, S: StorageEngine> EdgeMutation<'a, S> {
    pub fn set_label(mut self, label: impl Into<String>) -> Self {
        self.ops.push(EdgeOp::SetLabel(label.into()));
        self
    }

    pub fn set_property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.ops.push(EdgeOp::SetProperty(key.into(), value));
        self
    }

    pub fn remove_property(mut self, key: impl Into<String>) -> Self {
        self.ops.push(EdgeOp::RemoveProperty(key.into()));
        self
    }

    pub fn replace_properties(
        mut self,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Self {
        let props: std::collections::BTreeMap<String, Value> = properties.into_iter().collect();
        self.ops.push(EdgeOp::ReplaceProperties(props));
        self
    }

    pub fn apply(self) -> Result<()> {
        let current = self.db.get_edge(self.id)?;

        let mut label = current.label.clone();
        let mut properties = current.properties.clone();

        for op in &self.ops {
            match op {
                EdgeOp::SetLabel(l) => label = l.clone(),
                EdgeOp::SetProperty(k, v) => {
                    properties.insert(k.clone(), v.clone());
                }
                EdgeOp::RemoveProperty(k) => {
                    properties.remove(k);
                }
                EdgeOp::ReplaceProperties(props) => {
                    properties = props.clone();
                }
            }
        }

        self.db.storage.write(|txn| {
            if label != current.label {
                let old_out = EdgeIndex::out_key(current.from, &current.label, self.id);
                txn.delete(Db::OutEdges, &old_out)?;
                let old_in = EdgeIndex::in_key(current.to, &current.label, self.id);
                txn.delete(Db::InEdges, &old_in)?;

                let new_out = EdgeIndex::out_key(current.from, &label, self.id);
                txn.put(Db::OutEdges, &new_out, &self.id.to_be_bytes())?;
                let new_in = EdgeIndex::in_key(current.to, &label, self.id);
                txn.put(Db::InEdges, &new_in, &self.id.to_be_bytes())?;
            }

            let updated = Edge {
                id: self.id,
                from: current.from,
                to: current.to,
                label: label.clone(),
                properties,
            };

            let bytes =
                bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Edges, &self.id.to_be_bytes(), &bytes)?;

            Ok(())
        })
    }
}

fn validate_vector_dims(
    storage: &impl StorageEngine,
    label: &str,
    properties: &std::collections::BTreeMap<String, Value>,
) -> Result<()> {
    for (prop_name, prop_value) in properties {
        let Value::Vector(vector) = prop_value else {
            continue;
        };
        let Ok(meta) = VectorIndex::load_meta(storage, label, prop_name) else {
            continue;
        };
        if vector.len() != meta.dimension {
            return Err(HelixiteError::InvalidVectorDim {
                expected: meta.dimension,
                actual: vector.len(),
            });
        }
    }
    Ok(())
}

fn delete_node_indexes(
    txn: &mut dyn crate::storage::StorageTxn,
    old_label: &str,
    new_label: &str,
    id: NodeId,
    old_props: &std::collections::BTreeMap<String, Value>,
    new_props: &std::collections::BTreeMap<String, Value>,
) -> Result<()> {
    if old_label != new_label {
        let old_key = LabelIndex::key(old_label, id);
        txn.delete(Db::Labels, &old_key)?;
        let new_key = LabelIndex::key(new_label, id);
        txn.put(Db::Labels, &new_key, &[])?;

        for (prop_name, old_value) in old_props {
            if matches!(old_value, Value::Vector(_))
                && let Ok(meta) = VectorIndex::load_meta_from_txn(txn, old_label, prop_name)
            {
                VectorIndex::delete_from_txn(txn, old_label, prop_name, id, &meta)?;
            }
        }
    }

    for (prop_name, old_value) in old_props {
        let still_present = new_props.get(prop_name) == Some(old_value);
        if still_present {
            continue;
        }

        if let Some(key) = PropertyIndex::key(prop_name, old_value, id) {
            txn.delete(Db::Properties, &key)?;
        }

        if matches!(old_value, Value::Vector(_))
            && let Ok(meta) = VectorIndex::load_meta_from_txn(txn, old_label, prop_name)
        {
            VectorIndex::delete_from_txn(txn, old_label, prop_name, id, &meta)?;
        }
    }

    Ok(())
}

fn insert_node_indexes(
    txn: &mut dyn crate::storage::StorageTxn,
    label: &str,
    id: NodeId,
    properties: &std::collections::BTreeMap<String, Value>,
) -> Result<()> {
    for (prop_name, value) in properties {
        if let Some(key) = PropertyIndex::key(prop_name, value, id) {
            txn.put(Db::Properties, &key, &[])?;
        }

        if let Value::Vector(vector) = value {
            let Ok(meta) = VectorIndex::load_meta_from_txn(txn, label, prop_name) else {
                continue;
            };
            VectorIndex::delete_from_txn(txn, label, prop_name, id, &meta)?;
            let meta = VectorIndex::load_meta_from_txn(txn, label, prop_name)?;
            VectorIndex::insert_into_txn(txn, label, prop_name, id, vector, &meta)?;
        }
    }

    Ok(())
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

        validate_vector_dims(&self.storage, &label, &properties)?;

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

    pub fn node_mut(&self, id: NodeId) -> NodeMutation<'_, S> {
        NodeMutation {
            db: self,
            id,
            ops: Vec::new(),
        }
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

    pub fn edge_mut(&self, id: EdgeId) -> EdgeMutation<'_, S> {
        EdgeMutation {
            db: self,
            id,
            ops: Vec::new(),
        }
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
