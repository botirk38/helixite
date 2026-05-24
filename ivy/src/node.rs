use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::id::NodeId;
use crate::value::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub properties: BTreeMap<String, Value>,
}
