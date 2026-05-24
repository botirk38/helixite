use std::collections::BTreeMap;

use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::storage::StorageEngine;
use crate::storage::engine::{Db, Scan};
use crate::storage::{ReadTxn as StorageReadTxn, WriteTxn as StorageWriteTxn};
use crate::value::Value;

use crate::db::Helixite;
use crate::index::edges::EdgeIndex;
use crate::index::labels::EdgeLabelIndex;
use crate::index::nodes::NodeIndexes;
use crate::index::properties::EdgePropertyIndexes;
use crate::index::properties::NodePropertyIndexes;
use crate::index::properties::PropertyIndexRegistry;

pub struct WriteTxn<'a> {
    txn: &'a mut dyn StorageWriteTxn,
}

pub struct ReadTxn<'a> {
    txn: &'a dyn StorageReadTxn,
}

impl<'a> ReadTxn<'a> {
    pub(crate) fn new(txn: &'a dyn StorageReadTxn) -> Self {
        Self { txn }
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .txn
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    pub fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        let bytes = self
            .txn
            .get(Db::Edges, &id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(id))?;

        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    pub fn get_nodes(&self, ids: &[NodeId]) -> Result<Vec<Node>> {
        ids.iter().map(|id| self.get_node(*id)).collect()
    }

    pub fn get_edges(&self, ids: &[EdgeId]) -> Result<Vec<Edge>> {
        ids.iter().map(|id| self.get_edge(*id)).collect()
    }
}

impl<'a> WriteTxn<'a> {
    pub(crate) fn new(txn: &'a mut dyn StorageWriteTxn) -> Self {
        Self { txn }
    }

    pub fn add_node(
        &mut self,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<NodeId> {
        let label = label.into();
        let properties: BTreeMap<String, Value> = properties.into_iter().collect();

        NodeIndexes::validate_from_txn(self.txn, &label, &properties)?;
        NodePropertyIndexes::validate_unique_from_txn(self.txn, &label, &properties)?;

        let registered = PropertyIndexRegistry::load_nodes_from_txn(self.txn)?;

        let next_id = next_node_id(self.txn)?;

        let node = Node {
            id: next_id,
            label: label.clone(),
            properties: properties.clone(),
        };

        let bytes = bincode::serialize(&node).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.txn.put(Db::Nodes, &next_id.to_be_bytes(), &bytes)?;

        NodeIndexes::insert(self.txn, &label, next_id, &properties, &registered)?;

        Ok(next_id)
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        let bytes = self
            .txn
            .get(Db::Nodes, &id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(id))?;

        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    pub fn delete_node(&mut self, id: NodeId) -> Result<()> {
        let node = self.get_node(id)?;

        let node_registry = PropertyIndexRegistry::load_nodes_from_txn(self.txn)?;
        let edge_registry = PropertyIndexRegistry::load_edges_from_txn(self.txn)?;

        let out_prefix = EdgeIndex::out_prefix(id, None);
        let out_entries = self
            .txn
            .scan(Db::OutEdges, Scan::Prefix(&out_prefix), None)?;
        let out_keys: Vec<Vec<u8>> = out_entries.iter().map(|e| e.key.to_vec()).collect();
        for key in out_keys {
            let Some(edge_id) = EdgeIndex::decode_edge_id(&key) else {
                continue;
            };
            delete_edge_from_txn(self.txn, &edge_registry, edge_id)?;
        }

        let in_prefix = EdgeIndex::in_prefix(id, None);
        let in_entries = self.txn.scan(Db::InEdges, Scan::Prefix(&in_prefix), None)?;
        let in_keys: Vec<Vec<u8>> = in_entries.iter().map(|e| e.key.to_vec()).collect();
        for key in in_keys {
            let Some(edge_id) = EdgeIndex::decode_edge_id(&key) else {
                continue;
            };
            delete_edge_from_txn(self.txn, &edge_registry, edge_id)?;
        }

        NodeIndexes::delete(self.txn, &node, &node_registry)?;
        self.txn.delete(Db::Nodes, &node.id.to_be_bytes())?;

        Ok(())
    }

    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        label: impl Into<String>,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<EdgeId> {
        if self.txn.get(Db::Nodes, &from.to_be_bytes())?.is_none() {
            return Err(HelixiteError::NodeNotFound(from));
        }
        if self.txn.get(Db::Nodes, &to.to_be_bytes())?.is_none() {
            return Err(HelixiteError::NodeNotFound(to));
        }

        let label = label.into();
        let properties: BTreeMap<String, Value> = properties.into_iter().collect();

        let registered = PropertyIndexRegistry::load_edges_from_txn(self.txn)?;
        EdgePropertyIndexes::validate_unique_from_txn(self.txn, &label, &properties)?;

        let next_id = next_edge_id(self.txn)?;

        let edge = Edge {
            id: next_id,
            from,
            to,
            label: label.clone(),
            properties,
        };

        let bytes = bincode::serialize(&edge).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.txn.put(Db::Edges, &next_id.to_be_bytes(), &bytes)?;

        EdgeIndex::insert(self.txn, from, to, &label, next_id)?;
        EdgePropertyIndexes::insert(self.txn, &registered, &edge)?;

        let label_key = EdgeLabelIndex::key(&label, next_id);
        self.txn.put(Db::Labels, &label_key, &[])?;

        Ok(next_id)
    }

    pub fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        let bytes = self
            .txn
            .get(Db::Edges, &id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(id))?;

        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))
    }

    pub fn delete_edge(&mut self, id: EdgeId) -> Result<()> {
        let edge = self.get_edge(id)?;

        let registered = PropertyIndexRegistry::load_edges_from_txn(self.txn)?;

        let label_key = EdgeLabelIndex::key(&edge.label, edge.id);
        self.txn.delete(Db::Labels, &label_key)?;
        EdgeIndex::delete(self.txn, edge.from, edge.to, &edge.label, edge.id)?;
        EdgePropertyIndexes::delete(self.txn, &registered, &edge)?;
        self.txn.delete(Db::Edges, &edge.id.to_be_bytes())?;

        Ok(())
    }

    pub fn node(&mut self, id: NodeId) -> NodeMut<'_> {
        NodeMut::new(self.txn, id)
    }

    pub fn edge(&mut self, id: EdgeId) -> EdgeMut<'_> {
        EdgeMut::new(self.txn, id)
    }
}

enum MutOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(BTreeMap<String, Value>),
    SetFrom(NodeId),
    SetTo(NodeId),
}

macro_rules! impl_mut_builder_methods {
    () => {
        pub fn set_label(mut self, label: impl Into<String>) -> Self {
            self.ops.push(MutOp::SetLabel(label.into()));
            self
        }

        pub fn set_property(mut self, key: impl Into<String>, value: Value) -> Self {
            self.ops.push(MutOp::SetProperty(key.into(), value));
            self
        }

        pub fn remove_property(mut self, key: impl Into<String>) -> Self {
            self.ops.push(MutOp::RemoveProperty(key.into()));
            self
        }

        pub fn replace_properties(
            mut self,
            properties: impl IntoIterator<Item = (String, Value)>,
        ) -> Self {
            let props: BTreeMap<String, Value> = properties.into_iter().collect();
            self.ops.push(MutOp::ReplaceProperties(props));
            self
        }
    };
}

fn apply_ops(label: &mut String, properties: &mut BTreeMap<String, Value>, ops: &[MutOp]) {
    for op in ops {
        match op {
            MutOp::SetLabel(l) => *label = l.clone(),
            MutOp::SetProperty(k, v) => {
                properties.insert(k.clone(), v.clone());
            }
            MutOp::RemoveProperty(k) => {
                properties.remove(k);
            }
            MutOp::ReplaceProperties(props) => {
                *properties = props.clone();
            }
            MutOp::SetFrom(_) | MutOp::SetTo(_) => {}
        }
    }
}

pub struct NodeMut<'a> {
    txn: &'a mut dyn StorageWriteTxn,
    id: NodeId,
    ops: Vec<MutOp>,
}

impl<'a> NodeMut<'a> {
    fn new(txn: &'a mut dyn StorageWriteTxn, id: NodeId) -> Self {
        Self {
            txn,
            id,
            ops: Vec::new(),
        }
    }

    impl_mut_builder_methods!();

    pub fn apply(self) -> Result<()> {
        let current_bytes = self
            .txn
            .get(Db::Nodes, &self.id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(self.id))?;

        let current: Node = bincode::deserialize(&current_bytes)
            .map_err(|e| HelixiteError::Codec(e.to_string()))?;

        let mut label = current.label.clone();
        let mut properties = current.properties.clone();
        apply_ops(&mut label, &mut properties, &self.ops);

        NodeIndexes::validate_from_txn(self.txn, &label, &properties)?;
        NodePropertyIndexes::validate_unique_replace_from_txn(
            self.txn,
            self.id,
            &label,
            &properties,
        )?;

        let registered = PropertyIndexRegistry::load_nodes_from_txn(self.txn)?;

        let old_label = &current.label;

        NodeIndexes::replace(
            self.txn,
            old_label,
            &label,
            self.id,
            &current.properties,
            &properties,
            &registered,
        )?;

        let updated = Node {
            id: self.id,
            label,
            properties,
        };

        let bytes =
            bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.txn.put(Db::Nodes, &self.id.to_be_bytes(), &bytes)?;

        Ok(())
    }
}

pub struct EdgeMut<'a> {
    txn: &'a mut dyn StorageWriteTxn,
    id: EdgeId,
    ops: Vec<MutOp>,
}

impl<'a> EdgeMut<'a> {
    fn new(txn: &'a mut dyn StorageWriteTxn, id: EdgeId) -> Self {
        Self {
            txn,
            id,
            ops: Vec::new(),
        }
    }

    impl_mut_builder_methods!();

    pub fn set_from(mut self, node_id: NodeId) -> Self {
        self.ops.push(MutOp::SetFrom(node_id));
        self
    }

    pub fn set_to(mut self, node_id: NodeId) -> Self {
        self.ops.push(MutOp::SetTo(node_id));
        self
    }

    pub fn apply(self) -> Result<()> {
        let current_bytes = self
            .txn
            .get(Db::Edges, &self.id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(self.id))?;

        let current: Edge = bincode::deserialize(&current_bytes)
            .map_err(|e| HelixiteError::Codec(e.to_string()))?;

        let mut label = current.label.clone();
        let mut properties = current.properties.clone();
        let mut from = current.from;
        let mut to = current.to;
        apply_ops(&mut label, &mut properties, &self.ops);
        for op in &self.ops {
            match op {
                MutOp::SetFrom(node_id) => from = *node_id,
                MutOp::SetTo(node_id) => to = *node_id,
                MutOp::SetLabel(_)
                | MutOp::SetProperty(_, _)
                | MutOp::RemoveProperty(_)
                | MutOp::ReplaceProperties(_) => {}
            }
        }

        if self.txn.get(Db::Nodes, &from.to_be_bytes())?.is_none() {
            return Err(HelixiteError::NodeNotFound(from));
        }
        if self.txn.get(Db::Nodes, &to.to_be_bytes())?.is_none() {
            return Err(HelixiteError::NodeNotFound(to));
        }

        EdgePropertyIndexes::validate_unique_replace_from_txn(
            self.txn,
            self.id,
            &label,
            &properties,
        )?;

        let registered = PropertyIndexRegistry::load_edges_from_txn(self.txn)?;

        if label != current.label || from != current.from || to != current.to {
            EdgeIndex::delete(self.txn, current.from, current.to, &current.label, self.id)?;
            EdgeIndex::insert(self.txn, from, to, &label, self.id)?;
        }

        if label != current.label {
            let old_label_key = EdgeLabelIndex::key(&current.label, self.id);
            self.txn.delete(Db::Labels, &old_label_key)?;
            let new_label_key = EdgeLabelIndex::key(&label, self.id);
            self.txn.put(Db::Labels, &new_label_key, &[])?;
        }

        let updated = Edge {
            id: self.id,
            from,
            to,
            label,
            properties,
        };

        EdgePropertyIndexes::replace(self.txn, &registered, &current, &updated)?;

        let bytes =
            bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.txn.put(Db::Edges, &self.id.to_be_bytes(), &bytes)?;

        Ok(())
    }
}

fn delete_edge_from_txn(
    txn: &mut dyn StorageWriteTxn,
    edge_registry: &PropertyIndexRegistry,
    edge_id: EdgeId,
) -> Result<()> {
    let bytes = match txn.get(Db::Edges, &edge_id.to_be_bytes())? {
        Some(b) => b,
        None => return Ok(()),
    };

    let edge: Edge =
        bincode::deserialize(&bytes).map_err(|e| HelixiteError::Codec(e.to_string()))?;

    EdgeIndex::delete(txn, edge.from, edge.to, &edge.label, edge.id)?;
    EdgePropertyIndexes::delete(txn, edge_registry, &edge)?;

    let label_key = EdgeLabelIndex::key(&edge.label, edge.id);
    txn.delete(Db::Labels, &label_key)?;

    txn.delete(Db::Edges, &edge.id.to_be_bytes())?;

    Ok(())
}

fn next_node_id(txn: &mut dyn StorageWriteTxn) -> Result<NodeId> {
    next_id(txn, b"next_node_id", "next_node_id")
}

fn next_edge_id(txn: &mut dyn StorageWriteTxn) -> Result<EdgeId> {
    next_id(txn, b"next_edge_id", "next_edge_id")
}

fn next_id(txn: &mut dyn StorageWriteTxn, key: &[u8], name: &str) -> Result<u64> {
    let bytes = match txn.get(Db::Metadata, key)? {
        Some(b) => b,
        None => {
            txn.put(Db::Metadata, key, &2u64.to_be_bytes())?;
            return Ok(1);
        }
    };

    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| HelixiteError::Storage(format!("invalid {name} metadata value")))?;

    let next_id = u64::from_be_bytes(bytes);

    let following_id = next_id
        .checked_add(1)
        .ok_or_else(|| HelixiteError::Storage(format!("{name} overflow")))?;

    txn.put(Db::Metadata, key, &following_id.to_be_bytes())?;

    Ok(next_id)
}

pub struct NodeMutBuilder<'a, S: StorageEngine> {
    db: &'a Helixite<S>,
    id: NodeId,
    ops: Vec<MutOp>,
}

impl<S: StorageEngine> NodeMutBuilder<'_, S> {
    pub(crate) fn new(db: &Helixite<S>, id: NodeId) -> NodeMutBuilder<'_, S> {
        NodeMutBuilder {
            db,
            id,
            ops: Vec::new(),
        }
    }

    impl_mut_builder_methods!();

    pub fn apply(self) -> Result<()> {
        self.db.write_transaction(|tx| {
            let mut node_mut = tx.node(self.id);
            for op in self.ops {
                node_mut = match op {
                    MutOp::SetLabel(l) => node_mut.set_label(l),
                    MutOp::SetProperty(k, v) => node_mut.set_property(k, v),
                    MutOp::RemoveProperty(k) => node_mut.remove_property(k),
                    MutOp::ReplaceProperties(p) => node_mut.replace_properties(p),
                    MutOp::SetFrom(_) | MutOp::SetTo(_) => unreachable!(),
                };
            }
            node_mut.apply()
        })
    }
}

pub struct EdgeMutBuilder<'a, S: StorageEngine> {
    db: &'a Helixite<S>,
    id: EdgeId,
    ops: Vec<MutOp>,
}

impl<S: StorageEngine> EdgeMutBuilder<'_, S> {
    pub(crate) fn new(db: &Helixite<S>, id: EdgeId) -> EdgeMutBuilder<'_, S> {
        EdgeMutBuilder {
            db,
            id,
            ops: Vec::new(),
        }
    }

    impl_mut_builder_methods!();

    pub fn set_from(mut self, node_id: NodeId) -> Self {
        self.ops.push(MutOp::SetFrom(node_id));
        self
    }

    pub fn set_to(mut self, node_id: NodeId) -> Self {
        self.ops.push(MutOp::SetTo(node_id));
        self
    }

    pub fn apply(self) -> Result<()> {
        self.db.write_transaction(|tx| {
            let mut edge_mut = tx.edge(self.id);
            for op in self.ops {
                edge_mut = match op {
                    MutOp::SetLabel(l) => edge_mut.set_label(l),
                    MutOp::SetProperty(k, v) => edge_mut.set_property(k, v),
                    MutOp::RemoveProperty(k) => edge_mut.remove_property(k),
                    MutOp::ReplaceProperties(p) => edge_mut.replace_properties(p),
                    MutOp::SetFrom(id) => edge_mut.set_from(id),
                    MutOp::SetTo(id) => edge_mut.set_to(id),
                };
            }
            edge_mut.apply()
        })
    }
}
