use heed::types::Bytes;
use heed::{Database, Env};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::value::Value;
use crate::storage::env::open_env;

const METADATA_NEXT_NODE_ID: &[u8] = b"next_node_id";

pub struct Helixite {
    pub(crate) env: Env,
    pub(crate) path: PathBuf,
    pub(crate) nodes_db: Database<Bytes, Bytes>,
    pub(crate) metadata_db: Database<Bytes, Bytes>,
}

impl Helixite {
    pub fn open(path: impl AsRef<Path>, config: Option<Config>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let config = config.unwrap_or_default();
        let env = open_env(&path, &config)?;

        let mut wtxn = env.write_txn()?;
        let nodes_db: Database<Bytes, Bytes> = env.create_database(&mut wtxn, Some("nodes"))?;
        let metadata_db: Database<Bytes, Bytes> = env.create_database(&mut wtxn, Some("metadata"))?;
        wtxn.commit()?;

        Ok(Self { env, path, nodes_db, metadata_db })
    }

    pub fn add_node(
        &self,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<NodeId> {
        let mut wtxn = self.env.write_txn()?;

        let next_id = self
            .metadata_db
            .get(&wtxn, METADATA_NEXT_NODE_ID)?
            .map(|bytes| u64::from_be_bytes(bytes.try_into().unwrap()))
            .unwrap_or(1);

        let node = Node {
            id: next_id,
            label: label.into(),
            properties: properties.into_iter().collect(),
        };

        let bytes =
            bincode::serialize(&node).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.nodes_db.put(&mut wtxn, &next_id.to_be_bytes(), &bytes)?;

        self.metadata_db
            .put(&mut wtxn, METADATA_NEXT_NODE_ID, &(next_id + 1).to_be_bytes())?;

        wtxn.commit()?;

        Ok(next_id)
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let rtxn = self.env.read_txn()?;

        let bytes = self
            .nodes_db
            .get(&rtxn, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        let node =
            bincode::deserialize(bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

        Ok(node)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
