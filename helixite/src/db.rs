use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::engine::Db;
use crate::storage::lmdb::LmdbStorage;
use crate::storage::StorageEngine;
use crate::value::Value;

const METADATA_NEXT_NODE_ID: &[u8] = b"next_node_id";

pub struct Helixite<S: StorageEngine = LmdbStorage> {
    path: PathBuf,
    storage: S,
}

impl Helixite {
    pub fn open(path: impl AsRef<Path>, config: Option<Config>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let config = config.unwrap_or_default();
        let storage = LmdbStorage::open(&path, &config)?;

        Ok(Self {
            path,
            storage,
        })
    }

    pub fn add_node(
        &self,
        label: String,
        properties: Vec<(String, Value)>,
    ) -> Result<NodeId> {
        self.storage.write(|txn| {
            let next_id = txn
                .get(Db::Metadata, METADATA_NEXT_NODE_ID)?
                .map(|bytes| u64::from_be_bytes(bytes.try_into().unwrap()))
                .unwrap_or(1);

            let node = Node {
                id: next_id,
                label,
                properties: properties.into_iter().collect(),
            };

            let bytes =
                bincode::serialize(&node).map_err(|e| HelixiteError::Codec(e.to_string()))?;
            txn.put(Db::Nodes, &next_id.to_be_bytes(), &bytes)?;
            txn.put(Db::Metadata, METADATA_NEXT_NODE_ID, &(next_id + 1).to_be_bytes())?;

            Ok(next_id)
        })
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .storage
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        let node =
            bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(node)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
