use helixite::{HelixiteBuilder, HelixiteError};
use tempfile::tempdir;

#[test]
fn test_create_node_property_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();
}

#[test]
fn test_create_node_property_index_fails_for_missing_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db
        .indexes()
        .nodes()
        .create_property("NonExistent", "email");

    assert!(matches!(result, Err(HelixiteError::LabelNotFound(_))));
}

#[test]
fn test_create_duplicate_node_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    let result = db
        .indexes()
        .nodes()
        .create_property("User", "name");

    assert!(matches!(result, Err(HelixiteError::DuplicateKey(_))));
}

#[test]
fn test_drop_node_property_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.indexes()
        .nodes()
        .drop_property("User", "name")
        .unwrap();

    let result = db
        .indexes()
        .nodes()
        .drop_property("User", "name");

    assert!(matches!(result, Err(HelixiteError::IndexNotFound(_))));
}

#[test]
fn test_create_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", vec![("since".to_string(), helixite::Value::Int(2020))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();
}

#[test]
fn test_create_duplicate_edge_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", vec![("since".to_string(), helixite::Value::Int(2020))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    let result = db
        .indexes()
        .edges()
        .create_property("knows", "since");

    assert!(matches!(result, Err(HelixiteError::DuplicateKey(_))));
}

#[test]
fn test_drop_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", vec![("since".to_string(), helixite::Value::Int(2020))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    db.indexes()
        .edges()
        .drop_property("knows", "since")
        .unwrap();

    let result = db
        .indexes()
        .edges()
        .drop_property("knows", "since");

    assert!(matches!(result, Err(HelixiteError::IndexNotFound(_))));
}

#[test]
fn test_drop_nonexistent_node_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
        .unwrap();

    let result = db
        .indexes()
        .nodes()
        .drop_property("User", "email");

    assert!(matches!(result, Err(HelixiteError::IndexNotFound(_))));
}

#[test]
fn test_drop_nonexistent_edge_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", Vec::new()).unwrap();

    let result = db
        .indexes()
        .edges()
        .drop_property("knows", "weight");

    assert!(matches!(result, Err(HelixiteError::IndexNotFound(_))));
}

#[test]
fn test_create_node_property_index_without_nodes() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "email")
        .unwrap();
}

#[test]
fn test_drop_then_recreate_node_property_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.indexes()
        .nodes()
        .drop_property("User", "name")
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();
}

#[test]
fn test_drop_then_recreate_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", Vec::new()).unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    db.indexes()
        .edges()
        .drop_property("knows", "since")
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();
}

#[test]
fn test_multiple_node_property_indexes_for_same_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![
        ("name".to_string(), helixite::Value::String("Alice".to_string())),
        ("email".to_string(), helixite::Value::String("alice@example.com".to_string())),
    ]).unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "email")
        .unwrap();
}

#[test]
fn test_multiple_edge_property_indexes_for_same_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", vec![
        ("since".to_string(), helixite::Value::Int(2020)),
        ("weight".to_string(), helixite::Value::Float(0.5)),
    ]).unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();
}

#[test]
fn test_node_property_index_survives_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
            .unwrap();
        db.indexes()
            .nodes()
            .create_property("User", "name")
            .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();

    let result = db
        .indexes()
        .nodes()
        .drop_property("User", "name");

    assert!(result.is_ok());
}

#[test]
fn test_node_property_index_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.add_node("User", vec![("name".to_string(), helixite::Value::String("Alice".to_string()))])
            .unwrap();
        db.indexes()
            .nodes()
            .create_property("User", "name")
            .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();

    let result = db
        .indexes()
        .nodes()
        .create_property("User", "name");

    assert!(matches!(result, Err(HelixiteError::DuplicateKey(_))));
}

#[test]
fn test_edge_property_index_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        db.add_edge(from, to, "knows", vec![("since".to_string(), helixite::Value::Int(2020))])
            .unwrap();
        db.indexes()
            .edges()
            .create_property("knows", "since")
            .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();

    let result = db
        .indexes()
        .edges()
        .create_property("knows", "since");

    assert!(matches!(result, Err(HelixiteError::DuplicateKey(_))));
}
