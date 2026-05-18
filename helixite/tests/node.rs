use helixite::{Helixite, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_add_node_returns_id() {
    let dir = tempdir().unwrap();
    let db = Helixite::open(dir.path(), None).unwrap();

    let id = db.add_node("User".to_string(), Vec::new()).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_get_node() {
    let dir = tempdir().unwrap();
    let db = Helixite::open(dir.path(), None).unwrap();

    let id = db
        .add_node(
            "User".to_string(),
            vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("age".to_string(), Value::Int(30)),
            ],
        )
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.id, id);
    assert_eq!(node.label, "User");
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Alice".to_string()))
    );
    assert_eq!(node.properties.get("age"), Some(&Value::Int(30)));
}

#[test]
fn test_get_missing_node_returns_error() {
    let dir = tempdir().unwrap();
    let db = Helixite::open(dir.path(), None).unwrap();

    let result = db.get_node(999);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_node_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = Helixite::open(path, None).unwrap();
        db.add_node(
            "Chunk".to_string(),
            vec![("text".to_string(), Value::String("hello".to_string()))],
        )
        .unwrap();
    }

    let db = Helixite::open(path, None).unwrap();
    let node = db.get_node(1).unwrap();
    assert_eq!(node.label, "Chunk");
    assert_eq!(
        node.properties.get("text"),
        Some(&Value::String("hello".to_string()))
    );
}

#[test]
fn test_multiple_nodes_get_incrementing_ids() {
    let dir = tempdir().unwrap();
    let db = Helixite::open(dir.path(), None).unwrap();

    let id1 = db.add_node("A".to_string(), Vec::new()).unwrap();
    let id2 = db.add_node("B".to_string(), Vec::new()).unwrap();
    let id3 = db.add_node("C".to_string(), Vec::new()).unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}
