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
            Value::String(s) => {
                let mut key = Vec::with_capacity(1 + s.len());
                key.push(0);
                key.extend(s.as_bytes());
                Some(key)
            }
            Value::Int(n) => {
                let mut key = Vec::with_capacity(9);
                key.push(1);
                key.extend(n.to_be_bytes());
                Some(key)
            }
            Value::Float(f) => {
                let mut key = Vec::with_capacity(9);
                key.push(4);
                let canonical = if *f == 0.0 { 0.0 } else { *f };
                let bits = canonical.to_bits();
                let ordered = if bits >> 63 == 0 {
                    bits ^ (1 << 63)
                } else {
                    !bits
                };
                key.extend(ordered.to_be_bytes());
                Some(key)
            }
            Value::Bool(b) => Some(vec![2, if *b { 1 } else { 0 }]),
            Value::Bytes(b) => {
                let mut key = Vec::with_capacity(1 + b.len());
                key.push(3);
                key.extend(b);
                Some(key)
            }
            Value::Vector(_) => None,
        }
    }
}
