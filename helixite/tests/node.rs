use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_add_node_returns_id() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_get_node() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db.get_node(999);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_node_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.add_node(
            "Chunk",
            vec![("text".to_string(), Value::String("hello".to_string()))],
        )
        .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let id1 = db.add_node("A", Vec::new()).unwrap();
    let id2 = db.add_node("B", Vec::new()).unwrap();
    let id3 = db.add_node("C", Vec::new()).unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_update_node_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("age".to_string(), Value::Int(30)),
            ],
        )
        .unwrap();

    db.update_node(
        id,
        None::<String>,
        Some(vec![
            ("name".to_string(), Value::String("Bob".to_string())),
            ("city".to_string(), Value::String("NYC".to_string())),
        ]),
    )
    .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
    assert_eq!(
        node.properties.get("city"),
        Some(&Value::String("NYC".to_string()))
    );
    assert_eq!(node.properties.get("age"), None);
}

#[test]
fn test_update_node_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();

    db.update_node(id, Some("Person"), None::<Vec<(String, Value)>>)
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "Person");

    let user_ids = db.nodes().label("User").ids().unwrap();
    assert!(user_ids.is_empty());

    let person_ids = db.nodes().label("Person").ids().unwrap();
    assert_eq!(person_ids, vec![id]);
}

#[test]
fn test_update_node_label_and_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.update_node(
        id,
        Some("Person"),
        Some(vec![("name".to_string(), Value::String("Bob".to_string()))]),
    )
    .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "Person");
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_update_node_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        let id = db
            .add_node(
                "User",
                vec![("name".to_string(), Value::String("Alice".to_string()))],
            )
            .unwrap();

        db.update_node(
            id,
            Some("Person"),
            Some(vec![("name".to_string(), Value::String("Bob".to_string()))]),
        )
        .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let node = db.get_node(1).unwrap();
    assert_eq!(node.label, "Person");
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_update_nonexistent_node_errors() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db.update_node(999, None::<String>, None::<Vec<(String, Value)>>);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}
