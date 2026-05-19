use crate::error::{HelixiteError, Result};
use crate::storage::StorageEngine;
use crate::storage::engine::Db;

use crate::index::labels::LabelIndex;
use crate::index::vector::{HnswConfig, VectorIndex};

const NODE_PROPERTY_INDEX_PREFIX: &str = "node_property_index";
const EDGE_PROPERTY_INDEX_PREFIX: &str = "edge_property_index";

pub struct IndexManager<'a, S: StorageEngine> {
    storage: &'a S,
}

pub struct VectorIndexManager<'a, S: StorageEngine> {
    storage: &'a S,
}

pub struct NodeIndexManager<'a, S: StorageEngine> {
    storage: &'a S,
}

pub struct EdgeIndexManager<'a, S: StorageEngine> {
    storage: &'a S,
}

impl<'a, S: StorageEngine> IndexManager<'a, S> {
    pub(crate) fn new(storage: &'a S) -> Self {
        Self { storage }
    }

    pub fn vectors(&self) -> VectorIndexManager<'a, S> {
        VectorIndexManager {
            storage: self.storage,
        }
    }

    pub fn nodes(&self) -> NodeIndexManager<'a, S> {
        NodeIndexManager {
            storage: self.storage,
        }
    }

    pub fn edges(&self) -> EdgeIndexManager<'a, S> {
        EdgeIndexManager {
            storage: self.storage,
        }
    }
}

impl<'a, S: StorageEngine> VectorIndexManager<'a, S> {
    pub fn create(
        &self,
        label: &str,
        property: &str,
        dimension: usize,
        config: HnswConfig,
    ) -> Result<()> {
        VectorIndex::create(self.storage, label, property, dimension, config)
    }
}

impl<'a, S: StorageEngine> NodeIndexManager<'a, S> {
    pub fn create_property(&self, label: &str, property: &str) -> Result<()> {
        if !self.label_exists(label)? {
            return Err(HelixiteError::LabelNotFound(label.to_string()));
        }

        if self.property_index_exists(label, property)? {
            return Err(HelixiteError::DuplicateKey(format!(
                "node property index {label}::{property} already exists"
            )));
        }

        self.register_property_index(label, property)
    }

    pub fn drop_property(&self, label: &str, property: &str) -> Result<()> {
        if !self.property_index_exists(label, property)? {
            return Err(HelixiteError::IndexNotFound(format!(
                "node property index {label}::{property}"
            )));
        }

        self.unregister_property_index(label, property)
    }

    fn label_exists(&self, label: &str) -> Result<bool> {
        let prefix = LabelIndex::prefix(label);
        let entries = self.storage.scan_prefix(Db::Labels, &prefix)?;
        Ok(!entries.is_empty())
    }

    fn property_index_exists(&self, label: &str, property: &str) -> Result<bool> {
        let key = Self::property_index_key(label, property);
        match self.storage.get(Db::Metadata, &key)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn register_property_index(&self, label: &str, property: &str) -> Result<()> {
        let key = Self::property_index_key(label, property);
        self.storage
            .write(|txn| txn.put(Db::Metadata, &key, &[]))
    }

    fn unregister_property_index(&self, label: &str, property: &str) -> Result<()> {
        let key = Self::property_index_key(label, property);
        self.storage
            .write(|txn| txn.delete(Db::Metadata, &key))
    }

    fn property_index_key(label: &str, property: &str) -> Vec<u8> {
        format!("{}::{}::{}", NODE_PROPERTY_INDEX_PREFIX, label, property).into_bytes()
    }
}

impl<'a, S: StorageEngine> EdgeIndexManager<'a, S> {
    pub fn create_property(&self, label: &str, property: &str) -> Result<()> {
        if self.property_index_exists(label, property)? {
            return Err(HelixiteError::DuplicateKey(format!(
                "edge property index {label}::{property} already exists"
            )));
        }

        self.register_property_index(label, property)
    }

    pub fn drop_property(&self, label: &str, property: &str) -> Result<()> {
        if !self.property_index_exists(label, property)? {
            return Err(HelixiteError::IndexNotFound(format!(
                "edge property index {label}::{property}"
            )));
        }

        self.unregister_property_index(label, property)
    }

    fn property_index_exists(&self, label: &str, property: &str) -> Result<bool> {
        let key = Self::property_index_key(label, property);
        match self.storage.get(Db::Metadata, &key)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn register_property_index(&self, label: &str, property: &str) -> Result<()> {
        let key = Self::property_index_key(label, property);
        self.storage
            .write(|txn| txn.put(Db::Metadata, &key, &[]))
    }

    fn unregister_property_index(&self, label: &str, property: &str) -> Result<()> {
        let key = Self::property_index_key(label, property);
        self.storage
            .write(|txn| txn.delete(Db::Metadata, &key))
    }

    fn property_index_key(label: &str, property: &str) -> Vec<u8> {
        format!("{}::{}::{}", EDGE_PROPERTY_INDEX_PREFIX, label, property).into_bytes()
    }
}
