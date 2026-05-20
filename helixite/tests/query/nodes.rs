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

#[test]
fn test_nodes_first() {
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

    let node = db.nodes().label("User").first().unwrap();
    assert!(node.is_some());
    assert_eq!(node.unwrap().label, "User");
}

#[test]
fn test_nodes_first_empty() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let node = db.nodes().label("NonExistent").first().unwrap();
    assert!(node.is_none());
}

#[test]
fn test_nodes_limit() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    for i in 0..5 {
        db.add_node(
            "User",
            vec![("name".to_string(), Value::String(format!("User{i}")))],
        )
        .unwrap();
    }

    let nodes = db.nodes().label("User").limit(2).collect().unwrap();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn test_nodes_limit_with_filters() {
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
            ("name".to_string(), Value::String("Alice".to_string())),
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

    let nodes = db
        .nodes()
        .label("User")
        .eq("name", Value::String("Alice".to_string()))
        .limit(2)
        .collect()
        .unwrap();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn test_node_page_limit_conflict() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    let err = db.nodes().label("User").limit(10).page(2).unwrap_err();
    assert!(
        err.to_string()
            .contains("limit() cannot be used with page()")
    );
}

#[test]
fn test_node_page_first_page() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    for i in 0..5 {
        db.add_node(
            "User",
            vec![("name".to_string(), Value::String(format!("User{i}")))],
        )
        .unwrap();
    }

    let page = db.nodes().label("User").page(2).unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(page.next_cursor.is_some());
}

#[test]
fn test_node_page_second_page() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    for i in 0..5 {
        db.add_node(
            "User",
            vec![("name".to_string(), Value::String(format!("User{i}")))],
        )
        .unwrap();
    }

    let page1 = db.nodes().label("User").page(2).unwrap();
    assert_eq!(page1.items.len(), 2);
    let cursor = page1.next_cursor.unwrap();

    let page2 = db.nodes().label("User").after(cursor).page(2).unwrap();
    assert_eq!(page2.items.len(), 2);
    assert!(page2.next_cursor.is_some());

    let page1_ids: std::collections::HashSet<_> = page1.items.iter().map(|n| n.id).collect();
    let page2_ids: std::collections::HashSet<_> = page2.items.iter().map(|n| n.id).collect();
    assert!(page1_ids.is_disjoint(&page2_ids));
}

#[test]
fn test_node_page_last_page_no_next_cursor() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    for _i in 0..3 {
        db.add_node("User", Vec::new()).unwrap();
    }

    let page1 = db.nodes().label("User").page(2).unwrap();
    assert_eq!(page1.items.len(), 2);
    let cursor = page1.next_cursor.unwrap();

    let page2 = db.nodes().label("User").after(cursor).page(2).unwrap();
    assert_eq!(page2.items.len(), 1);
    assert!(page2.next_cursor.is_none());
}

#[test]
fn test_node_page_invalid_cursor_format() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    let err = db
        .nodes()
        .label("User")
        .after("bad_cursor")
        .page(2)
        .unwrap_err();
    assert!(err.to_string().contains("node cursor must start with 'n:'"));
}

#[test]
fn test_node_page_stale_cursor() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();
    db.add_node("User", Vec::new()).unwrap();

    let stale_cursor = "n:999999";

    let err = db
        .nodes()
        .label("User")
        .after(stale_cursor)
        .page(2)
        .unwrap_err();
    assert!(err.to_string().contains("cursor not found"));
}

#[test]
fn test_node_count_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    for _i in 0..5 {
        db.add_node("User", Vec::new()).unwrap();
    }

    let err = db.nodes().label("User").after("n:1").count().unwrap_err();
    assert!(err.to_string().contains("after() requires page()"));
}

#[test]
fn test_node_page_zero_size_rejected() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    let err = db.nodes().label("User").page(0).unwrap_err();
    assert!(err.to_string().contains("page size must be greater than 0"));
}

#[test]
fn test_node_collect_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    let err = db.nodes().label("User").after("n:1").collect().unwrap_err();
    assert!(err.to_string().contains("after() requires page()"));
}

#[test]
fn test_node_ids_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.add_node("User", Vec::new()).unwrap();

    let err = db.nodes().label("User").after("n:1").ids().unwrap_err();
    assert!(err.to_string().contains("after() requires page()"));
}
