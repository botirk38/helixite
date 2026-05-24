use ivy::{IvyBuilder, IvyError};
use tempfile::tempdir;

#[test]
fn test_create_node_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![(
            "name".to_string(),
            ivy::Value::String("Alice".to_string()),
        )],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();
}

#[test]
fn test_create_node_property_index_fails_for_missing_label() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let result = db.indexes().nodes().create_property("NonExistent", "email");

    assert!(matches!(result, Err(IvyError::LabelNotFound(_))));
}

#[test]
fn test_create_duplicate_node_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![(
            "name".to_string(),
            ivy::Value::String("Alice".to_string()),
        )],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    let result = db.indexes().nodes().create_property("User", "name");

    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_create_unique_node_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        [(
            "email".to_string(),
            ivy::Value::String("a@example.com".into()),
        )],
    )
    .unwrap();
    db.add_node(
        "User",
        [(
            "email".to_string(),
            ivy::Value::String("b@example.com".into()),
        )],
    )
    .unwrap();

    db.indexes().nodes().create_unique("User", "email").unwrap();

    let result = db.add_node(
        "User",
        [(
            "email".to_string(),
            ivy::Value::String("a@example.com".into()),
        )],
    );
    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_create_unique_node_property_index_fails_with_existing_duplicate() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        [(
            "email".to_string(),
            ivy::Value::String("a@example.com".into()),
        )],
    )
    .unwrap();
    db.add_node(
        "User",
        [(
            "email".to_string(),
            ivy::Value::String("a@example.com".into()),
        )],
    )
    .unwrap();

    let result = db.indexes().nodes().create_unique("User", "email");
    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_unique_node_property_blocks_update_duplicate() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        [(
            "email".to_string(),
            ivy::Value::String("a@example.com".into()),
        )],
    )
    .unwrap();
    let b = db
        .add_node(
            "User",
            [(
                "email".to_string(),
                ivy::Value::String("b@example.com".into()),
            )],
        )
        .unwrap();

    db.indexes().nodes().create_unique("User", "email").unwrap();

    let result = db
        .update_node(b)
        .set_property("email", ivy::Value::String("a@example.com".into()))
        .apply();
    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_drop_node_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![(
            "name".to_string(),
            ivy::Value::String("Alice".to_string()),
        )],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.indexes().nodes().drop_property("User", "name").unwrap();

    let result = db.indexes().nodes().drop_property("User", "name");

    assert!(matches!(result, Err(IvyError::IndexNotFound(_))));
}

#[test]
fn test_create_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(
        from,
        to,
        "knows",
        vec![("since".to_string(), ivy::Value::Int(2020))],
    )
    .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();
}

#[test]
fn test_create_duplicate_edge_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(
        from,
        to,
        "knows",
        vec![("since".to_string(), ivy::Value::Int(2020))],
    )
    .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    let result = db.indexes().edges().create_property("knows", "since");

    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_create_unique_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    db.add_edge(
        from,
        to,
        "knows",
        [(
            "external_id".to_string(),
            ivy::Value::String("edge-a".into()),
        )],
    )
    .unwrap();
    db.indexes()
        .edges()
        .create_unique("knows", "external_id")
        .unwrap();

    let result = db.add_edge(
        from,
        to,
        "knows",
        [(
            "external_id".to_string(),
            ivy::Value::String("edge-a".into()),
        )],
    );
    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_unique_edge_property_blocks_update_duplicate() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    db.add_edge(
        from,
        to,
        "knows",
        [(
            "external_id".to_string(),
            ivy::Value::String("edge-a".into()),
        )],
    )
    .unwrap();
    let edge_b = db
        .add_edge(
            from,
            to,
            "knows",
            [(
                "external_id".to_string(),
                ivy::Value::String("edge-b".into()),
            )],
        )
        .unwrap();

    db.indexes()
        .edges()
        .create_unique("knows", "external_id")
        .unwrap();

    let result = db
        .update_edge(edge_b)
        .set_property("external_id", ivy::Value::String("edge-a".into()))
        .apply();
    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_drop_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(
        from,
        to,
        "knows",
        vec![("since".to_string(), ivy::Value::Int(2020))],
    )
    .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    db.indexes()
        .edges()
        .drop_property("knows", "since")
        .unwrap();

    let result = db.indexes().edges().drop_property("knows", "since");

    assert!(matches!(result, Err(IvyError::IndexNotFound(_))));
}

#[test]
fn test_drop_nonexistent_node_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![(
            "name".to_string(),
            ivy::Value::String("Alice".to_string()),
        )],
    )
    .unwrap();

    let result = db.indexes().nodes().drop_property("User", "email");

    assert!(matches!(result, Err(IvyError::IndexNotFound(_))));
}

#[test]
fn test_drop_nonexistent_edge_property_index_fails() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(from, to, "knows", Vec::new()).unwrap();

    let result = db.indexes().edges().drop_property("knows", "weight");

    assert!(matches!(result, Err(IvyError::IndexNotFound(_))));
}

#[test]
fn test_create_node_property_index_without_nodes() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "email")
        .unwrap();
}

#[test]
fn test_drop_then_recreate_node_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![(
            "name".to_string(),
            ivy::Value::String("Alice".to_string()),
        )],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.indexes().nodes().drop_property("User", "name").unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();
}

#[test]
fn test_drop_then_recreate_edge_property_index() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

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
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![
            (
                "name".to_string(),
                ivy::Value::String("Alice".to_string()),
            ),
            (
                "email".to_string(),
                ivy::Value::String("alice@example.com".to_string()),
            ),
        ],
    )
    .unwrap();

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
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(
        from,
        to,
        "knows",
        vec![
            ("since".to_string(), ivy::Value::Int(2020)),
            ("weight".to_string(), ivy::Value::Float(0.5)),
        ],
    )
    .unwrap();

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
        let db = IvyBuilder::new().open(path).unwrap();
        db.add_node(
            "User",
            vec![(
                "name".to_string(),
                ivy::Value::String("Alice".to_string()),
            )],
        )
        .unwrap();
        db.indexes()
            .nodes()
            .create_property("User", "name")
            .unwrap();
    }

    let db = IvyBuilder::new().open(path).unwrap();

    let result = db.indexes().nodes().drop_property("User", "name");

    assert!(result.is_ok());
}

#[test]
fn test_node_property_index_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = IvyBuilder::new().open(path).unwrap();
        db.add_node(
            "User",
            vec![(
                "name".to_string(),
                ivy::Value::String("Alice".to_string()),
            )],
        )
        .unwrap();
        db.indexes()
            .nodes()
            .create_property("User", "name")
            .unwrap();
    }

    let db = IvyBuilder::new().open(path).unwrap();

    let result = db.indexes().nodes().create_property("User", "name");

    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}

#[test]
fn test_edge_property_index_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = IvyBuilder::new().open(path).unwrap();
        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        db.add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), ivy::Value::Int(2020))],
        )
        .unwrap();
        db.indexes()
            .edges()
            .create_property("knows", "since")
            .unwrap();
    }

    let db = IvyBuilder::new().open(path).unwrap();

    let result = db.indexes().edges().create_property("knows", "since");

    assert!(matches!(result, Err(IvyError::DuplicateKey(_))));
}
