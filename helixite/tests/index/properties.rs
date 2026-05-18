use helixite::index::properties::PropertyIndex;
use helixite::value::Value;

#[test]
fn test_property_key_string_value() {
    let key = PropertyIndex::key("name", &Value::String("Alice".to_string()), 42);
    assert!(key.is_some());
    let key = key.unwrap();
    assert!(!key.is_empty());

    let decoded = PropertyIndex::decode_node_id(&key).unwrap();
    assert_eq!(decoded, 42);
}

#[test]
fn test_property_key_int_value() {
    let key = PropertyIndex::key("age", &Value::Int(30), 99);
    assert!(key.is_some());
    let key = key.unwrap();
    assert!(!key.is_empty());

    let decoded = PropertyIndex::decode_node_id(&key).unwrap();
    assert_eq!(decoded, 99);
}

#[test]
fn test_property_key_bool_value() {
    let key = PropertyIndex::key("active", &Value::Bool(true), 7);
    assert!(key.is_some());
    let key = key.unwrap();
    assert!(!key.is_empty());

    let decoded = PropertyIndex::decode_node_id(&key).unwrap();
    assert_eq!(decoded, 7);
}

#[test]
fn test_property_key_float_returns_none() {
    let key = PropertyIndex::key("score", &Value::Float(0.5), 1);
    assert!(key.is_none());
}

#[test]
fn test_property_prefix() {
    let prefix = PropertyIndex::prefix("name", &Value::String("Alice".to_string()));
    assert!(prefix.is_some());
    let prefix = prefix.unwrap();
    assert!(!prefix.is_empty());

    let key_a = PropertyIndex::key("name", &Value::String("Alice".to_string()), 1).unwrap();
    let key_b = PropertyIndex::key("name", &Value::String("Alice".to_string()), 99).unwrap();
    assert!(key_a.starts_with(&prefix));
    assert!(key_b.starts_with(&prefix));
}

#[test]
fn test_different_properties_dont_collide() {
    let key_a = PropertyIndex::key("name", &Value::String("Alice".to_string()), 1).unwrap();
    let key_b = PropertyIndex::key("age", &Value::Int(30), 1).unwrap();
    assert_ne!(key_a, key_b);
}

#[test]
fn test_same_property_different_values_dont_collide() {
    let key_a = PropertyIndex::key("name", &Value::String("Alice".to_string()), 1).unwrap();
    let key_b = PropertyIndex::key("name", &Value::String("Bob".to_string()), 1).unwrap();
    assert_ne!(key_a, key_b);
}

#[test]
fn test_decode_corrupt_key() {
    assert!(PropertyIndex::decode_node_id(&[0, 0, 0]).is_none());
}
