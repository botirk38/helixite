use std::collections::BTreeMap;

use crate::error::{HelixiteError, Result};
use crate::id::NodeId;
use crate::node::Node;
use crate::storage::ReadTxn;
use crate::storage::WriteTxn;
use crate::storage::engine::Db;
use crate::value::Value;

use super::labels::NodeLabelIndex;
use super::properties::NodePropertyIndex;
use super::properties::NodePropertyIndexes;
use super::properties::PropertyIndexRegistry;
use super::vector::VectorIndex;

pub(crate) struct NodeIndexes;

impl NodeIndexes {
    pub(crate) fn validate_from_txn(
        txn: &dyn ReadTxn,
        label: &str,
        properties: &BTreeMap<String, Value>,
    ) -> Result<()> {
        for (prop_name, prop_value) in properties {
            let Value::Vector(vector) = prop_value else {
                continue;
            };
            let Ok(meta) = VectorIndex::load_meta(txn, label, prop_name) else {
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

    pub(crate) fn insert(
        txn: &mut dyn WriteTxn,
        label: &str,
        id: NodeId,
        properties: &BTreeMap<String, Value>,
        registered_indexes: &PropertyIndexRegistry,
    ) -> Result<()> {
        let label_key = NodeLabelIndex::key(label, id);
        txn.put(Db::Labels, &label_key, &[])?;

        for (prop_name, value) in properties {
            if Self::is_indexed(registered_indexes, label, prop_name)
                && let Some(key) = NodePropertyIndex::key(label, prop_name, value, id)
            {
                txn.put(Db::Properties, &key, &[])?;
            }
            if let Value::Vector(vector) = value {
                let Ok(meta) = VectorIndex::load_meta(txn, label, prop_name) else {
                    continue;
                };
                VectorIndex::insert(txn, label, prop_name, id, vector, &meta)?;
            }
        }

        Ok(())
    }

    pub(crate) fn replace(
        txn: &mut dyn WriteTxn,
        old_label: &str,
        new_label: &str,
        id: NodeId,
        old_props: &BTreeMap<String, Value>,
        new_props: &BTreeMap<String, Value>,
        registered_indexes: &PropertyIndexRegistry,
    ) -> Result<()> {
        if old_label != new_label {
            let old_key = NodeLabelIndex::key(old_label, id);
            txn.delete(Db::Labels, &old_key)?;
            let new_key = NodeLabelIndex::key(new_label, id);
            txn.put(Db::Labels, &new_key, &[])?;

            for (prop_name, old_value) in old_props {
                if matches!(old_value, Value::Vector(_))
                    && let Ok(meta) = VectorIndex::load_meta(txn, old_label, prop_name)
                {
                    VectorIndex::delete(txn, old_label, prop_name, id, &meta)?;
                }
            }

            for (prop_name, value) in new_props {
                if let Value::Vector(vector) = value {
                    let Ok(meta) = VectorIndex::load_meta(txn, new_label, prop_name) else {
                        continue;
                    };
                    VectorIndex::insert(txn, new_label, prop_name, id, vector, &meta)?;
                }
            }

            for (prop_name, old_value) in old_props {
                if Self::is_indexed(registered_indexes, old_label, prop_name)
                    && let Some(key) = NodePropertyIndex::key(old_label, prop_name, old_value, id)
                {
                    txn.delete(Db::Properties, &key)?;
                }
            }

            for (prop_name, new_value) in new_props {
                if Self::is_indexed(registered_indexes, new_label, prop_name)
                    && let Some(key) = NodePropertyIndex::key(new_label, prop_name, new_value, id)
                {
                    txn.put(Db::Properties, &key, &[])?;
                }
            }
        } else {
            for (prop_name, old_value) in old_props {
                let still_present = new_props.get(prop_name) == Some(old_value);
                if still_present {
                    continue;
                }

                if Self::is_indexed(registered_indexes, old_label, prop_name)
                    && let Some(key) = NodePropertyIndex::key(old_label, prop_name, old_value, id)
                {
                    txn.delete(Db::Properties, &key)?;
                }

                if matches!(old_value, Value::Vector(_))
                    && let Ok(meta) = VectorIndex::load_meta(txn, old_label, prop_name)
                {
                    VectorIndex::delete(txn, old_label, prop_name, id, &meta)?;
                }
            }

            for (prop_name, new_value) in new_props {
                let was_present = old_props.get(prop_name) == Some(new_value);
                if was_present {
                    continue;
                }

                if Self::is_indexed(registered_indexes, new_label, prop_name)
                    && let Some(key) = NodePropertyIndex::key(new_label, prop_name, new_value, id)
                {
                    txn.put(Db::Properties, &key, &[])?;
                }

                if let Value::Vector(vector) = new_value {
                    let Ok(meta) = VectorIndex::load_meta(txn, new_label, prop_name) else {
                        continue;
                    };
                    VectorIndex::delete(txn, new_label, prop_name, id, &meta)?;
                    let meta = VectorIndex::load_meta(txn, new_label, prop_name)?;
                    VectorIndex::insert(txn, new_label, prop_name, id, vector, &meta)?;
                }
            }
        }

        Ok(())
    }

    fn is_indexed(registered: &PropertyIndexRegistry, label: &str, property: &str) -> bool {
        registered.contains(label, property)
    }

    pub(crate) fn delete(
        txn: &mut dyn WriteTxn,
        node: &Node,
        registered_indexes: &PropertyIndexRegistry,
    ) -> Result<()> {
        let label_key = NodeLabelIndex::key(&node.label, node.id);
        txn.delete(Db::Labels, &label_key)?;

        for (prop_name, value) in &node.properties {
            if matches!(value, Value::Vector(_))
                && let Ok(meta) = VectorIndex::load_meta(txn, &node.label, prop_name)
            {
                VectorIndex::delete(txn, &node.label, prop_name, node.id, &meta)?;
            }
        }

        NodePropertyIndexes::delete(txn, registered_indexes, node)?;

        Ok(())
    }
}
