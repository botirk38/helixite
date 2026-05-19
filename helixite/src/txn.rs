use std::collections::BTreeMap;

use crate::edge::Edge;
use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};
use crate::node::Node;
use crate::storage::StorageTxn;
use crate::storage::engine::Db;
use crate::value::Value;

use crate::index::edges::EdgeIndex;
use crate::index::nodes::NodeIndexes;
use crate::index::properties::EdgePropertyIndexes;
use crate::index::properties::PropertyIndexRegistry;

pub struct WriteTxn<'a> {
    txn: &'a mut dyn StorageTxn,
}

impl<'a> WriteTxn<'a> {
    pub(crate) fn new(txn: &'a mut dyn StorageTxn) -> Self {
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
        let out_entries = self.txn.scan_prefix(Db::OutEdges, &out_prefix)?;

        for (key, _) in &out_entries {
            let Some(edge_id) = EdgeIndex::decode_edge_id(key) else {
                continue;
            };
            delete_edge_from_txn(self.txn, &edge_registry, edge_id)?;
        }

        let in_prefix = EdgeIndex::in_prefix(id, None);
        let in_entries = self.txn.scan_prefix(Db::InEdges, &in_prefix)?;

        for (key, _) in &in_entries {
            let Some(edge_id) = EdgeIndex::decode_edge_id(key) else {
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
        let label = label.into();
        let properties: BTreeMap<String, Value> = properties.into_iter().collect();

        let registered = PropertyIndexRegistry::load_edges_from_txn(self.txn)?;

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

pub struct NodeMut<'a> {
    txn: &'a mut dyn StorageTxn,
    id: NodeId,
    ops: Vec<NodeOp>,
}

enum NodeOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(BTreeMap<String, Value>),
}

impl<'a> NodeMut<'a> {
    fn new(txn: &'a mut dyn StorageTxn, id: NodeId) -> Self {
        Self {
            txn,
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
        let current_bytes = self
            .txn
            .get(Db::Nodes, &self.id.to_be_bytes())?
            .ok_or(HelixiteError::NodeNotFound(self.id))?;

        let current: Node = bincode::deserialize(&current_bytes)
            .map_err(|e| HelixiteError::Codec(e.to_string()))?;

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

        NodeIndexes::validate_from_txn(self.txn, &label, &properties)?;

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
            label: label.clone(),
            properties: properties.clone(),
        };

        let bytes =
            bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.txn.put(Db::Nodes, &self.id.to_be_bytes(), &bytes)?;

        Ok(())
    }
}

pub struct EdgeMut<'a> {
    txn: &'a mut dyn StorageTxn,
    id: EdgeId,
    ops: Vec<EdgeOp>,
}

enum EdgeOp {
    SetLabel(String),
    SetProperty(String, Value),
    RemoveProperty(String),
    ReplaceProperties(BTreeMap<String, Value>),
}

impl<'a> EdgeMut<'a> {
    fn new(txn: &'a mut dyn StorageTxn, id: EdgeId) -> Self {
        Self {
            txn,
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
        let current_bytes = self
            .txn
            .get(Db::Edges, &self.id.to_be_bytes())?
            .ok_or(HelixiteError::EdgeNotFound(self.id))?;

        let current: Edge = bincode::deserialize(&current_bytes)
            .map_err(|e| HelixiteError::Codec(e.to_string()))?;

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

        let registered = PropertyIndexRegistry::load_edges_from_txn(self.txn)?;

        if label != current.label {
            EdgeIndex::replace_label(
                self.txn,
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

        EdgePropertyIndexes::replace(self.txn, &registered, &current, &updated)?;

        let bytes =
            bincode::serialize(&updated).map_err(|e| HelixiteError::Codec(e.to_string()))?;
        self.txn.put(Db::Edges, &self.id.to_be_bytes(), &bytes)?;

        Ok(())
    }
}

fn delete_edge_from_txn(
    txn: &mut dyn StorageTxn,
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
    txn.delete(Db::Edges, &edge.id.to_be_bytes())?;

    Ok(())
}

fn next_node_id(txn: &mut dyn StorageTxn) -> Result<NodeId> {
    next_id(txn, b"next_node_id", "next_node_id")
}

fn next_edge_id(txn: &mut dyn StorageTxn) -> Result<EdgeId> {
    next_id(txn, b"next_edge_id", "next_edge_id")
}

fn next_id(txn: &mut dyn StorageTxn, key: &[u8], name: &str) -> Result<u64> {
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
