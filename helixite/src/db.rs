use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::storage::lmdb::LmdbStorage;
use crate::value::Value;

const METADATA_NEXT_NODE_ID: &[u8] = b"next_node_id";

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
        self.storage.write(|txn| {
            let next_id = match txn.get(Db::Metadata, METADATA_NEXT_NODE_ID)? {
                Some(bytes) => decode_u64(&bytes, "next_node_id")?,
                None => 1,
            };

            let node = Node {
                id: next_id,
                label,
                properties: properties.into_iter().collect(),
            };

            let bytes =
                bincode::serialize(&node).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Nodes, &next_id.to_be_bytes(), &bytes)?;
            txn.put(
                Db::Metadata,
                METADATA_NEXT_NODE_ID,
                &(next_id + 1).to_be_bytes(),
            )?;

            Ok(next_id)
        })
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .storage
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        let node = bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(node)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }
}

fn decode_u64(bytes: &[u8], name: &str) -> Result<u64> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| HelixiteError::Storage(format!("invalid {name} metadata value")))?;
    Ok(u64::from_be_bytes(bytes))
}
