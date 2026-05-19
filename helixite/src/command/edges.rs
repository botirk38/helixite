use std::collections::BTreeMap;

use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::EdgeId;
use crate::storage::StorageEngine;
use crate::storage::engine::Db;
use crate::value::Value;

use crate::index::edges::EdgeIndex;

pub struct EdgeCommand<'a, S: StorageEngine> {
    db: &'a crate::db::Helixite<S>,
    id: EdgeId,
    ops: Vec<EdgeOp>,
}

enum EdgeOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(BTreeMap<String, Value>),
}

impl<'a, S: StorageEngine> EdgeCommand<'a, S> {
    pub(crate) fn new(db: &'a crate::db::Helixite<S>, id: EdgeId) -> Self {
        Self {
            db,
            id,
            ops: Vec::new(),
        }
    }

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
        let props: BTreeMap<String, Value> = properties.into_iter().collect();
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

        self.db.storage().write(|txn| {
            if label != current.label {
                EdgeIndex::replace_label(
                    txn,
                    current.from,
                    current.to,
                    &current.label,
                    &label,
                    self.id,
                )?;
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
