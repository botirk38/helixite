use serde::{Deserialize, Serialize};

use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    Vector(Vec<f32>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum IndexedValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
}

impl Eq for IndexedValue {}

impl PartialOrd for IndexedValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexedValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_index_key().cmp(&other.to_index_key())
    }
}

impl IndexedValue {
    fn to_index_key(&self) -> Vec<u8> {
        match self {
            IndexedValue::String(s) => {
                let mut key = Vec::with_capacity(1 + s.len());
                key.push(0);
                key.extend(s.as_bytes());
                key
            }
            IndexedValue::Int(n) => {
                let mut key = Vec::with_capacity(9);
                key.push(1);
                key.extend(n.to_be_bytes());
                key
            }
            IndexedValue::Float(f) => {
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
                key
            }
            IndexedValue::Bool(b) => vec![2, if *b { 1 } else { 0 }],
            IndexedValue::Bytes(b) => {
                let mut key = Vec::with_capacity(1 + b.len());
                key.push(3);
                key.extend(b);
                key
            }
        }
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl From<Vec<f32>> for Value {
    fn from(v: Vec<f32>) -> Self {
        Value::Vector(v)
    }
}

impl Value {
    pub(crate) fn to_index_key(&self) -> Option<Vec<u8>> {
        self.to_indexed_value()
            .map(|indexed_value| indexed_value.to_index_key())
    }

    pub(crate) fn to_indexed_value(&self) -> Option<IndexedValue> {
        match self {
            Value::String(s) => Some(IndexedValue::String(s.clone())),
            Value::Int(n) => Some(IndexedValue::Int(*n)),
            Value::Float(f) if !f.is_nan() => Some(IndexedValue::Float(*f)),
            Value::Float(_) => None,
            Value::Bool(b) => Some(IndexedValue::Bool(*b)),
            Value::Bytes(b) => Some(IndexedValue::Bytes(b.clone())),
            Value::Vector(_) => None,
        }
    }

    pub(crate) fn from_index_key(key: &[u8]) -> Option<IndexedValue> {
        let (kind, value) = key.split_first()?;
        match kind {
            0 => String::from_utf8(value.to_vec())
                .ok()
                .map(IndexedValue::String),
            1 => {
                let bytes: [u8; 8] = value.try_into().ok()?;
                Some(IndexedValue::Int(i64::from_be_bytes(bytes)))
            }
            2 => match value {
                [0] => Some(IndexedValue::Bool(false)),
                [1] => Some(IndexedValue::Bool(true)),
                _ => None,
            },
            3 => Some(IndexedValue::Bytes(value.to_vec())),
            4 => {
                let bytes: [u8; 8] = value.try_into().ok()?;
                let ordered = u64::from_be_bytes(bytes);
                let bits = if ordered >> 63 == 1 {
                    ordered ^ (1 << 63)
                } else {
                    !ordered
                };
                Some(IndexedValue::Float(f64::from_bits(bits)))
            }
            _ => None,
        }
    }
}
