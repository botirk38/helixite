use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    Vector(Vec<f32>),
}

impl Value {
    pub(crate) fn to_index_key(&self) -> Option<Vec<u8>> {
        match self {
            Value::String(s) => Some(s.as_bytes().to_vec()),
            Value::Int(n) => Some(n.to_be_bytes().to_vec()),
            Value::Float(f) => Some(f.to_be_bytes().to_vec()),
            Value::Bool(b) => Some(vec![if *b { 1 } else { 0 }]),
            Value::Bytes(b) => Some(b.clone()),
            Value::Vector(_) => None,
        }
    }
}
