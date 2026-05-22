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
        self.to_bytes().cmp(&other.to_bytes())
    }
}

impl IndexedValue {
    const SIGN_BIT: u64 = 1 << 63;

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
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
                let ordered = (*n as u64) ^ Self::SIGN_BIT;
                key.extend(ordered.to_be_bytes());
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

    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (kind, value) = bytes.split_first()?;
        match kind {
            0 => String::from_utf8(value.to_vec()).ok().map(Self::String),
            1 => {
                let bytes: [u8; 8] = value.try_into().ok()?;
                let ordered = u64::from_be_bytes(bytes);
                let raw = ordered ^ Self::SIGN_BIT;
                Some(Self::Int(i64::from_be_bytes(raw.to_be_bytes())))
            }
            2 => match value {
                [0] => Some(Self::Bool(false)),
                [1] => Some(Self::Bool(true)),
                _ => None,
            },
            3 => Some(Self::Bytes(value.to_vec())),
            4 => {
                let bytes: [u8; 8] = value.try_into().ok()?;
                let ordered = u64::from_be_bytes(bytes);
                let bits = if ordered >> 63 == 1 {
                    ordered ^ (1 << 63)
                } else {
                    !ordered
                };
                Some(Self::Float(f64::from_bits(bits)))
            }
            _ => None,
        }
    }

    pub(crate) fn compare_same_type(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::String(left), Self::String(right)) => Some(left.cmp(right)),
            (Self::Int(left), Self::Int(right)) => Some(left.cmp(right)),
            (Self::Float(left), Self::Float(right)) => Some(left.total_cmp(right)),
            (Self::Bool(left), Self::Bool(right)) => Some(left.cmp(right)),
            (Self::Bytes(left), Self::Bytes(right)) => Some(left.cmp(right)),
            _ => None,
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
            .map(|indexed_value| indexed_value.to_bytes())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── IndexedValue roundtrip: to_bytes → from_bytes ──

    #[test]
    fn indexed_value_roundtrip_string() {
        let original = IndexedValue::String("hello".into());
        let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn indexed_value_roundtrip_int() {
        for n in [0i64, 1, -1, 42, -42, 1000000] {
            let original = IndexedValue::Int(n);
            let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn indexed_value_roundtrip_float() {
        for f in [0.0f64, 1.0, -1.0, 42.5, -42.5, 0.001] {
            let original = IndexedValue::Float(f);
            let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn indexed_value_roundtrip_bool() {
        for b in [true, false] {
            let original = IndexedValue::Bool(b);
            let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn indexed_value_roundtrip_bytes() {
        let original = IndexedValue::Bytes(vec![0, 1, 255]);
        let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
        assert_eq!(original, restored);
    }

    // ── Boundary values ──

    #[test]
    fn indexed_value_int_min_max_roundtrip() {
        for n in [i64::MIN, i64::MAX] {
            let original = IndexedValue::Int(n);
            let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn indexed_value_float_infinity_roundtrip() {
        for f in [f64::INFINITY, f64::NEG_INFINITY] {
            let original = IndexedValue::Float(f);
            let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn indexed_value_float_negative_zero_canonical() {
        let neg_zero = IndexedValue::Float(-0.0);
        let pos_zero = IndexedValue::Float(0.0);
        assert_eq!(neg_zero.to_bytes(), pos_zero.to_bytes());
    }

    #[test]
    fn indexed_value_empty_string_roundtrip() {
        let original = IndexedValue::String(String::new());
        let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn indexed_value_empty_bytes_roundtrip() {
        let original = IndexedValue::Bytes(vec![]);
        let restored = IndexedValue::from_bytes(&original.to_bytes()).unwrap();
        assert_eq!(original, restored);
    }

    // ── from_bytes with invalid input ──

    #[test]
    fn indexed_value_from_empty_bytes_returns_none() {
        assert!(IndexedValue::from_bytes(&[]).is_none());
    }

    #[test]
    fn indexed_value_from_unknown_tag_returns_none() {
        assert!(IndexedValue::from_bytes(&[255, 0, 0]).is_none());
    }

    #[test]
    fn indexed_value_int_from_truncated_bytes_returns_none() {
        assert!(IndexedValue::from_bytes(&[1, 0, 0]).is_none());
    }

    #[test]
    fn indexed_value_bool_from_invalid_payload_returns_none() {
        assert!(IndexedValue::from_bytes(&[2, 2]).is_none());
    }

    // ── Ordering correctness via to_bytes ──

    #[test]
    fn indexed_value_int_ordering_preserves_sign() {
        let neg = IndexedValue::Int(-100);
        let zero = IndexedValue::Int(0);
        let pos = IndexedValue::Int(100);
        assert!(neg.to_bytes() < zero.to_bytes());
        assert!(zero.to_bytes() < pos.to_bytes());
    }

    #[test]
    fn indexed_value_int_ordering_min_to_max() {
        let min = IndexedValue::Int(i64::MIN);
        let neg = IndexedValue::Int(-1);
        let zero = IndexedValue::Int(0);
        let pos = IndexedValue::Int(1);
        let max = IndexedValue::Int(i64::MAX);

        let mut bytes: Vec<Vec<u8>> = [min, neg, zero, pos, max]
            .iter()
            .map(IndexedValue::to_bytes)
            .collect();
        let sorted = bytes.clone();
        bytes.sort();
        assert_eq!(bytes, sorted);
    }

    #[test]
    fn indexed_value_float_ordering() {
        let neg = IndexedValue::Float(-1.0);
        let zero = IndexedValue::Float(0.0);
        let pos = IndexedValue::Float(1.0);
        assert!(neg.to_bytes() < zero.to_bytes());
        assert!(zero.to_bytes() < pos.to_bytes());
    }

    #[test]
    fn indexed_value_float_ordering_full_range() {
        let values = vec![
            IndexedValue::Float(f64::NEG_INFINITY),
            IndexedValue::Float(-1e308),
            IndexedValue::Float(-1.0),
            IndexedValue::Float(-f64::MIN_POSITIVE),
            IndexedValue::Float(0.0),
            IndexedValue::Float(f64::MIN_POSITIVE),
            IndexedValue::Float(1.0),
            IndexedValue::Float(1e308),
            IndexedValue::Float(f64::INFINITY),
        ];

        let mut bytes: Vec<Vec<u8>> = values.iter().map(IndexedValue::to_bytes).collect();
        let sorted = bytes.clone();
        bytes.sort();
        assert_eq!(bytes, sorted);
    }

    #[test]
    fn indexed_value_string_ordering() {
        let a = IndexedValue::String("apple".into());
        let b = IndexedValue::String("banana".into());
        assert!(a.to_bytes() < b.to_bytes());
    }

    // ── compare_same_type ──

    #[test]
    fn compare_same_type_cross_type_returns_none() {
        let int = IndexedValue::Int(1);
        let string = IndexedValue::String("1".into());
        assert!(int.compare_same_type(&string).is_none());
    }

    #[test]
    fn compare_same_type_int_ordering() {
        let a = IndexedValue::Int(10);
        let b = IndexedValue::Int(20);
        assert_eq!(a.compare_same_type(&b), Some(std::cmp::Ordering::Less));
    }

    #[test]
    fn compare_same_type_float_ordering() {
        let a = IndexedValue::Float(1.0);
        let b = IndexedValue::Float(2.0);
        assert_eq!(a.compare_same_type(&b), Some(std::cmp::Ordering::Less));
    }

    // ── Value::to_indexed_value / to_index_key ──

    #[test]
    fn value_to_indexed_value_nan_returns_none() {
        assert!(Value::Float(f64::NAN).to_indexed_value().is_none());
    }

    #[test]
    fn value_to_indexed_value_vector_returns_none() {
        assert!(Value::Vector(vec![1.0]).to_indexed_value().is_none());
    }

    #[test]
    fn value_to_index_key_nan_returns_none() {
        assert!(Value::Float(f64::NAN).to_index_key().is_none());
    }

    #[test]
    fn value_to_index_key_vector_returns_none() {
        assert!(Value::Vector(vec![1.0]).to_index_key().is_none());
    }

    #[test]
    fn value_to_index_key_returns_some_for_indexable_types() {
        assert!(Value::String("hi".into()).to_index_key().is_some());
        assert!(Value::Int(42).to_index_key().is_some());
        assert!(Value::Float(1.0).to_index_key().is_some());
        assert!(Value::Bool(true).to_index_key().is_some());
        assert!(Value::Bytes(vec![1]).to_index_key().is_some());
    }

    // ── From trait implementations ──

    #[test]
    fn value_from_string_owned() {
        let v: Value = String::from("hello").into();
        assert_eq!(v, Value::String("hello".into()));
    }

    #[test]
    fn value_from_str_ref() {
        let v: Value = "hello".into();
        assert_eq!(v, Value::String("hello".into()));
    }

    #[test]
    fn value_from_i64() {
        let v: Value = 42i64.into();
        assert_eq!(v, Value::Int(42));
    }

    #[test]
    fn value_from_f64() {
        let v: Value = 2.72f64.into();
        assert_eq!(v, Value::Float(2.72));
    }

    #[test]
    fn value_from_bool() {
        let v: Value = true.into();
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn value_from_vec_u8() {
        let v: Value = vec![1u8, 2, 3].into();
        assert_eq!(v, Value::Bytes(vec![1, 2, 3]));
    }

    #[test]
    fn value_from_vec_f32() {
        let v: Value = vec![0.1f32, 0.2].into();
        assert_eq!(v, Value::Vector(vec![0.1, 0.2]));
    }
}
