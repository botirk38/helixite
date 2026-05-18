use helixite::{HelixiteBuilder, Value};
use tempfile::tempdir;

#[test]
fn test_nodes_by_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db.nodes().label("NonExistent").collect().unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_nodes_by_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    let alices = db
        .nodes()
        .where_eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(alices.len(), 2);

    let bob = db
        .nodes()
        .where_eq("name", Value::String("Bob".to_string()))
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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    let users = db
        .nodes()
        .label("User")
        .where_eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].label, "User");

    let posts = db
        .nodes()
        .label("Post")
        .where_eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].label, "Post");
}

#[test]
fn test_nodes_count() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    let result = db
        .nodes()
        .where_eq("name", Value::String("Alice".to_string()))
        .where_eq("age", Value::Int(30))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].properties.get("city"),
        Some(&Value::String("NYC".to_string()))
    );

    let result = db
        .nodes()
        .where_eq("age", Value::Int(30))
        .where_eq("city", Value::String("NYC".to_string()))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 2);

    let result = db
        .nodes()
        .where_eq("name", Value::String("Alice".to_string()))
        .where_eq("age", Value::Int(30))
        .where_eq("city", Value::String("NYC".to_string()))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 1);

    let result = db
        .nodes()
        .where_eq("name", Value::String("Alice".to_string()))
        .where_eq("age", Value::Int(99))
        .collect()
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_nodes_property_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
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
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let users = db.nodes().label("User").collect().unwrap();
    assert_eq!(users.len(), 2);

    let alices = db
        .nodes()
        .where_eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(alices.len(), 1);
}
