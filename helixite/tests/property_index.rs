use helixite::{HelixiteBuilder, Value};
use tempfile::tempdir;

#[test]
fn test_find_nodes_by_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), Value::String("Alice".to_string()))])
        .unwrap();
    db.add_node("User", vec![("name".to_string(), Value::String("Bob".to_string()))])
        .unwrap();
    db.add_node("Post", vec![("title".to_string(), Value::String("Hello".to_string()))])
        .unwrap();

    let users = db.find_nodes_by_label("User").unwrap();
    assert_eq!(users.len(), 2);

    let posts = db.find_nodes_by_label("Post").unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].label, "Post");
}

#[test]
fn test_find_nodes_by_label_empty() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db.find_nodes_by_label("NonExistent").unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_nodes_by_property_string() {
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
        .find_nodes_by_property("User", "name", &Value::String("Alice".to_string()))
        .unwrap();
    assert_eq!(alices.len(), 2);

    let bob = db
        .find_nodes_by_property("User", "name", &Value::String("Bob".to_string()))
        .unwrap();
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].properties.get("name"), Some(&Value::String("Bob".to_string())));
}

#[test]
fn test_find_nodes_by_property_int() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("age".to_string(), Value::Int(30))])
        .unwrap();
    db.add_node("User", vec![("age".to_string(), Value::Int(25))])
        .unwrap();
    db.add_node("User", vec![("age".to_string(), Value::Int(30))])
        .unwrap();

    let result = db
        .find_nodes_by_property("User", "age", &Value::Int(30))
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_find_nodes_by_property_no_match() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), Value::String("Alice".to_string()))])
        .unwrap();

    let result = db
        .find_nodes_by_property("User", "name", &Value::String("Charlie".to_string()))
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_nodes_by_property_different_labels() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("name".to_string(), Value::String("Alice".to_string()))])
        .unwrap();
    db.add_node("Post", vec![("name".to_string(), Value::String("Alice".to_string()))])
        .unwrap();

    let users = db
        .find_nodes_by_property("User", "name", &Value::String("Alice".to_string()))
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].label, "User");

    let posts = db
        .find_nodes_by_property("Post", "name", &Value::String("Alice".to_string()))
        .unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].label, "Post");
}

#[test]
fn test_find_nodes_by_property_bool() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node("User", vec![("active".to_string(), Value::Bool(true))])
        .unwrap();
    db.add_node("User", vec![("active".to_string(), Value::Bool(false))])
        .unwrap();
    db.add_node("User", vec![("active".to_string(), Value::Bool(true))])
        .unwrap();

    let active = db
        .find_nodes_by_property("User", "active", &Value::Bool(true))
        .unwrap();
    assert_eq!(active.len(), 2);
}

#[test]
fn test_find_nodes_by_property_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.add_node("User", vec![("name".to_string(), Value::String("Alice".to_string()))])
            .unwrap();
        db.add_node("User", vec![("name".to_string(), Value::String("Bob".to_string()))])
            .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let users = db.find_nodes_by_label("User").unwrap();
    assert_eq!(users.len(), 2);

    let alices = db
        .find_nodes_by_property("User", "name", &Value::String("Alice".to_string()))
        .unwrap();
    assert_eq!(alices.len(), 1);
}
