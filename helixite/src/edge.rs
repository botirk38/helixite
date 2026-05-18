use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::id::{EdgeId, NodeId};
use crate::value::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Out,
    In,
}
