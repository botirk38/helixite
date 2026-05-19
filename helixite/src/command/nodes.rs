use std::collections::BTreeMap;

use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::value::Value;

use crate::index::nodes::NodeIndexes;
use crate::index::properties::PropertyIndexRegistry;

pub struct NodeCommand<'a, S: StorageEngine> {
    db: &'a crate::db::Helixite<S>,
    id: NodeId,
    ops: Vec<NodeOp>,
}

enum NodeOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(BTreeMap<String, Value>),
}

impl<'a, S: StorageEngine> NodeCommand<'a, S> {
    pub(crate) fn new(db: &'a crate::db::Helixite<S>, id: NodeId) -> Self {
        Self {
            db,
            id,
            ops: Vec::new(),
        }
    }

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
        let props: BTreeMap<String, Value> = properties.into_iter().collect();
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

        NodeIndexes::validate(self.db.storage(), &label, &properties)?;

        self.db.storage().write(|txn| {
            let registered = PropertyIndexRegistry::load_nodes_from_txn(txn)?;

            let old_label = &current.label;

            NodeIndexes::replace(
                txn,
                old_label,
                &label,
                self.id,
                &current.properties,
                &properties,
                &registered,
            )?;

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
