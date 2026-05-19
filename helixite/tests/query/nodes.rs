use helixite::{HelixiteBuilder, Value};
use tempfile::tempdir;

#[test]
fn test_nodes_by_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![("name".to_string(), Value::String("Alice".to_string()))],
    )
    .unwrap();
    db.add_node(
        "User",
        vec![("name".to_string(), Value::String("Bob".to_string()))],
    )
    .unwrap();
    db.add_node(
        "Post",
        vec![("title".to_string(), Value::String("Hello".to_string()))],
    )
    .unwrap();

    let users = db.nodes().label("User").collect().unwrap();
    assert_eq!(users.len(), 2);

    let posts = db.nodes().label("Post").collect().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].label, "Post");
}

#[test]
fn test_nodes_by_label_empty() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.nodes().label("NonExistent").collect().unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_nodes_by_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(30)),
        ],
    )
    .unwrap();
    db.add_node(
        "User",
        vec![
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Int(25)),
        ],
    )
    .unwrap();
    db.add_node(
        "User",
        vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(35)),
        ],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    let alices = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(alices.len(), 2);

    let bob = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Bob".to_string()))
        .collect()
        .unwrap();
    assert_eq!(bob.len(), 1);
    assert_eq!(
        bob[0].properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_nodes_by_property_with_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![("name".to_string(), Value::String("Alice".to_string()))],
    )
    .unwrap();
    db.add_node(
        "Post",
        vec![("name".to_string(), Value::String("Alice".to_string()))],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();
    db.indexes()
        .nodes()
        .create_property("Post", "name")
        .unwrap();

    let users = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].label, "User");

    let posts = db
        .nodes()
        .label("Post")
        .eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].label, "Post");
}

#[test]
fn test_nodes_count() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();
    db.add_node("User", Vec::new()).unwrap();
    db.add_node("Post", Vec::new()).unwrap();

    let user_count = db.nodes().label("User").count().unwrap();
    assert_eq!(user_count, 2);

    let post_count = db.nodes().label("Post").count().unwrap();
    assert_eq!(post_count, 1);
}

#[test]
fn test_nodes_ids() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id1 = db.add_node("User", Vec::new()).unwrap();
    let id2 = db.add_node("User", Vec::new()).unwrap();

    let ids = db.nodes().label("User").ids().unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));
}

#[test]
fn test_multi_index_intersection() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node(
        "User",
        vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(30)),
            ("city".to_string(), Value::String("NYC".to_string())),
        ],
    )
    .unwrap();
    db.add_node(
        "User",
        vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(25)),
            ("city".to_string(), Value::String("LA".to_string())),
        ],
    )
    .unwrap();
    db.add_node(
        "User",
        vec![
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Int(30)),
            ("city".to_string(), Value::String("NYC".to_string())),
        ],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();
    db.indexes().nodes().create_property("User", "age").unwrap();
    db.indexes()
        .nodes()
        .create_property("User", "city")
        .unwrap();

    let result = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .eq("age", Value::Int(30))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].properties.get("city"),
        Some(&Value::String("NYC".to_string()))
    );

    let result = db
        .nodes()
        .label("User")
        .eq("age", Value::Int(30))
        .eq("city", Value::String("NYC".to_string()))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 2);

    let result = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .eq("age", Value::Int(30))
        .eq("city", Value::String("NYC".to_string()))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 1);

    let result = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .eq("age", Value::Int(99))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_nodes_property_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();
        db.add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();
        db.add_node(
            "User",
            vec![("name".to_string(), Value::String("Bob".to_string()))],
        )
        .unwrap();
        db.indexes()
            .nodes()
            .create_property("User", "name")
            .unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();
    let users = db.nodes().label("User").collect().unwrap();
    assert_eq!(users.len(), 2);

    let alices = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(alices.len(), 1);
}

#[test]
fn test_node_label_change_preserves_indexed_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.add_node("Person", Vec::new()).unwrap();
    db.indexes()
        .nodes()
        .create_property("Person", "name")
        .unwrap();

    db.write(|tx| tx.node(id).set_label("Person").apply())
        .unwrap();

    let users = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert!(users.is_empty());

    let persons = db
        .nodes()
        .label("Person")
        .eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0].id, id);
}

#[test]
fn test_node_label_change_with_both_indexes() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "name")
        .unwrap();

    db.add_node("Person", Vec::new()).unwrap();
    db.indexes()
        .nodes()
        .create_property("Person", "name")
        .unwrap();

    db.write(|tx| tx.node(id).set_label("Person").apply())
        .unwrap();

    let persons = db
        .nodes()
        .label("Person")
        .eq("name", Value::String("Alice".to_string()))
        .count()
        .unwrap();
    assert_eq!(persons, 1);

    let users = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .count()
        .unwrap();
    assert_eq!(users, 0);
}

#[test]
fn test_float_negative_zero_indexed_as_positive_zero() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "Measurement",
            vec![("value".to_string(), Value::Float(-0.0))],
        )
        .unwrap();

    db.indexes()
        .nodes()
        .create_property("Measurement", "value")
        .unwrap();

    let results = db
        .nodes()
        .label("Measurement")
        .eq("value", Value::Float(0.0))
        .collect()
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
}
