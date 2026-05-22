use std::cmp::Ordering;

use crate::value::{IndexedValue, Value};

#[derive(Debug, Clone)]
pub(crate) enum PropertyFilter {
    Eq(String, Value),
    Ne(String, Value),
    Gt(String, Value),
    Gte(String, Value),
    Lt(String, Value),
    Lte(String, Value),
    In(String, Vec<Value>),
}

impl PropertyFilter {
    pub(crate) fn property(&self) -> &str {
        match self {
            PropertyFilter::Eq(property, _)
            | PropertyFilter::Ne(property, _)
            | PropertyFilter::Gt(property, _)
            | PropertyFilter::Gte(property, _)
            | PropertyFilter::Lt(property, _)
            | PropertyFilter::Lte(property, _)
            | PropertyFilter::In(property, _) => property,
        }
    }

    pub(crate) fn matches_indexed(&self, indexed_value: &IndexedValue) -> bool {
        match self {
            PropertyFilter::Eq(_, value) => {
                value.to_indexed_value().as_ref() == Some(indexed_value)
            }
            PropertyFilter::Ne(_, value) => {
                value.to_indexed_value().as_ref() != Some(indexed_value)
            }
            PropertyFilter::Gt(_, value) => value
                .to_indexed_value()
                .and_then(|filter_value| indexed_value.compare_same_type(&filter_value))
                .is_some_and(Ordering::is_gt),
            PropertyFilter::Gte(_, value) => value
                .to_indexed_value()
                .and_then(|filter_value| indexed_value.compare_same_type(&filter_value))
                .is_some_and(Ordering::is_ge),
            PropertyFilter::Lt(_, value) => value
                .to_indexed_value()
                .and_then(|filter_value| indexed_value.compare_same_type(&filter_value))
                .is_some_and(Ordering::is_lt),
            PropertyFilter::Lte(_, value) => value
                .to_indexed_value()
                .and_then(|filter_value| indexed_value.compare_same_type(&filter_value))
                .is_some_and(Ordering::is_le),
            PropertyFilter::In(_, values) => values
                .iter()
                .filter_map(Value::to_indexed_value)
                .any(|filter_value| indexed_value == &filter_value),
        }
    }
}
