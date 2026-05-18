use helixite::Value;

#[test]
fn test_value_string() {
    let v = Value::String("hello".into());
    assert!(matches!(v, Value::String(s) if s == "hello"));
}

#[test]
fn test_value_int() {
    let v = Value::Int(42);
    assert!(matches!(v, Value::Int(42)));
}

#[test]
fn test_value_float() {
    let v = Value::Float(42.5);
    assert!(matches!(v, Value::Float(f) if (f - 42.5).abs() < f64::EPSILON));
}

#[test]
fn test_value_bool() {
    let v = Value::Bool(true);
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn test_value_bytes() {
    let v = Value::Bytes(vec![1, 2, 3]);
    assert!(matches!(v, Value::Bytes(b) if b == vec![1, 2, 3]));
}

#[test]
fn test_value_vector() {
    let v = Value::Vector(vec![0.1, 0.2, 0.3]);
    assert!(matches!(v, Value::Vector(v) if v.len() == 3));
}

#[test]
fn test_value_clone() {
    let v1 = Value::String("test".into());
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}

#[test]
fn test_value_debug() {
    let v = Value::Int(10);
    let debug = format!("{v:?}");
    assert!(debug.contains("Int"));
}

#[test]
fn test_value_equality() {
    let v1 = Value::Int(10);
    let v2 = Value::Int(10);
    let v3 = Value::Int(20);
    assert_eq!(v1, v2);
    assert_ne!(v1, v3);
}

#[test]
fn test_value_serde_roundtrip() {
    let values = vec![
        Value::String("hello".into()),
        Value::Int(42),
        Value::Float(42.5),
        Value::Bool(true),
        Value::Bytes(vec![1, 2, 3]),
        Value::Vector(vec![0.1, 0.2]),
    ];

    for v in values {
        let bytes = bincode::serialize(&v).unwrap();
        let restored: Value = bincode::deserialize(&bytes).unwrap();
        assert_eq!(v, restored);
    }
}
