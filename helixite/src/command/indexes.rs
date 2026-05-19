use crate::error::Result;
use crate::storage::StorageEngine;

use crate::index::properties::EdgePropertyIndexes;
use crate::index::properties::NodePropertyIndexes;
use crate::index::vector::{HnswConfig, VectorIndex};

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

    pub fn drop(&self, label: &str, property: &str) -> Result<()> {
        VectorIndex::drop(self.storage, label, property)
    }
}

impl<'a, S: StorageEngine> NodeIndexManager<'a, S> {
    pub fn create_property(&self, label: &str, property: &str) -> Result<()> {
        NodePropertyIndexes::create(self.storage, label, property)
    }

    pub fn drop_property(&self, label: &str, property: &str) -> Result<()> {
        NodePropertyIndexes::drop(self.storage, label, property)
    }
}

impl<'a, S: StorageEngine> EdgeIndexManager<'a, S> {
    pub fn create_property(&self, label: &str, property: &str) -> Result<()> {
        EdgePropertyIndexes::create(self.storage, label, property)
    }

    pub fn drop_property(&self, label: &str, property: &str) -> Result<()> {
        EdgePropertyIndexes::drop(self.storage, label, property)
    }
}
