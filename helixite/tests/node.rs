use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_add_node_returns_id() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_get_node() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

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
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.get_node(999);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_node_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();
        db.add_node(
            "Chunk",
            vec![("text".to_string(), Value::String("hello".to_string()))],
        )
        .unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();
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
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id1 = db.add_node("A", Vec::new()).unwrap();
    let id2 = db.add_node("B", Vec::new()).unwrap();
    let id3 = db.add_node("C", Vec::new()).unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_mutate_node_set_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.node_mut(id)
        .set_property("name", Value::String("Bob".into()))
        .apply()
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_mutate_node_remove_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("age".to_string(), Value::Int(30)),
            ],
        )
        .unwrap();

    db.node_mut(id).remove_property("age").apply().unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.properties.get("age"), None);
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Alice".to_string()))
    );
}

#[test]
fn test_mutate_node_replace_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("age".to_string(), Value::Int(30)),
            ],
        )
        .unwrap();

    db.node_mut(id)
        .replace_properties(vec![
            ("name".to_string(), Value::String("Bob".into())),
            ("city".to_string(), Value::String("NYC".into())),
        ])
        .apply()
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
fn test_mutate_node_set_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();

    db.node_mut(id).set_label("Person").apply().unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "Person");

    let user_ids = db.nodes().label("User").ids().unwrap();
    assert!(user_ids.is_empty());

    let person_ids = db.nodes().label("Person").ids().unwrap();
    assert_eq!(person_ids, vec![id]);
}

#[test]
fn test_mutate_node_label_and_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.node_mut(id)
        .set_label("Person")
        .set_property("name", Value::String("Bob".into()))
        .apply()
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "Person");
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_mutate_node_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();
        let id = db
            .add_node(
                "User",
                vec![("name".to_string(), Value::String("Alice".to_string()))],
            )
            .unwrap();

        db.node_mut(id)
            .set_label("Person")
            .set_property("name", Value::String("Bob".into()))
            .apply()
            .unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();
    let node = db.get_node(1).unwrap();
    assert_eq!(node.label, "Person");
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_mutate_nonexistent_node_errors() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.node_mut(999).apply();
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_mutate_node_empty_apply_is_noop() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.node_mut(id).apply().unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "User");
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Alice".to_string()))
    );
}

#[test]
fn test_mutate_node_replace_properties_wins_over_set_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.node_mut(id)
        .set_property("name", Value::String("Bob".into()))
        .replace_properties(vec![("name".to_string(), Value::String("Charlie".into()))])
        .apply()
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Charlie".to_string()))
    );
}

#[test]
fn test_mutate_node_set_property_wins_over_remove_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.node_mut(id)
        .remove_property("name")
        .set_property("name", Value::String("Bob".into()))
        .apply()
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Bob".to_string()))
    );
}

#[test]
fn test_mutate_node_label_remove_property_set_property_combined() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("age".to_string(), Value::Int(30)),
            ],
        )
        .unwrap();

    db.node_mut(id)
        .set_label("Person")
        .remove_property("age")
        .set_property("city", Value::String("NYC".into()))
        .apply()
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "Person");
    assert_eq!(node.properties.get("age"), None);
    assert_eq!(
        node.properties.get("name"),
        Some(&Value::String("Alice".to_string()))
    );
    assert_eq!(
        node.properties.get("city"),
        Some(&Value::String("NYC".to_string()))
    );

    let user_ids = db.nodes().label("User").ids().unwrap();
    assert!(user_ids.is_empty());

    let person_ids = db.nodes().label("Person").ids().unwrap();
    assert_eq!(person_ids, vec![id]);
}

#[test]
fn test_mutate_node_label_with_vector_and_scalar_property_change() {
    use helixite::HnswConfig;

    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.indexes()
        .vectors()
        .create("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();
    db.indexes()
        .vectors()
        .create("Doc", "embedding", 3, HnswConfig::default())
        .unwrap();

    let id = db
        .add_node(
            "Chunk",
            vec![
                ("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0])),
                ("title".to_string(), Value::String("Intro".to_string())),
            ],
        )
        .unwrap();

    db.node_mut(id)
        .set_label("Doc")
        .set_property("title", Value::String("Updated".into()))
        .remove_property("title")
        .apply()
        .unwrap();

    let node = db.get_node(id).unwrap();
    assert_eq!(node.label, "Doc");
    assert_eq!(node.properties.get("title"), None);

    let chunk_ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();
    assert_eq!(chunk_ids.len(), 0);

    let doc_ids = db
        .nodes()
        .label("Doc")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();
    assert_eq!(doc_ids.len(), 1);
    assert_eq!(doc_ids[0], id);
}

#[test]
fn test_delete_node() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".to_string()))],
        )
        .unwrap();

    db.delete_node(id).unwrap();

    let result = db.get_node(id);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(_))));
}

#[test]
fn test_delete_nonexistent_node_errors() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.delete_node(999);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_delete_node_cascades_to_outgoing_edges() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    db.delete_node(a).unwrap();

    let out = db.node(b).in_("knows").collect_edges().unwrap();
    assert!(out.is_empty());

    let out = db.node(c).in_("knows").collect_edges().unwrap();
    assert!(out.is_empty());
}

#[test]
fn test_delete_node_cascades_to_incoming_edges() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    db.delete_node(c).unwrap();

    let out = db.node(a).out("knows").collect_edges().unwrap();
    assert!(out.is_empty());

    let out = db.node(b).out("knows").collect_edges().unwrap();
    assert!(out.is_empty());
}

#[test]
fn test_delete_node_removes_from_label_query() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();
    db.add_node("User", Vec::new()).unwrap();

    let users = db.nodes().label("User").collect().unwrap();
    assert_eq!(users.len(), 2);

    db.delete_node(id).unwrap();

    let users = db.nodes().label("User").collect().unwrap();
    assert_eq!(users.len(), 1);
}

#[test]
fn test_delete_node_removes_indexed_property_entries() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
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

    let alices = db
        .nodes()
        .label("User")
        .where_eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(alices.len(), 1);

    db.delete_node(id).unwrap();

    let alices = db
        .nodes()
        .label("User")
        .where_eq("name", Value::String("Alice".to_string()))
        .collect()
        .unwrap();
    assert_eq!(alices.len(), 0);
}

#[test]
fn test_delete_node_cascades_to_indexed_edges() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let edges = db
        .node(a)
        .out("knows")
        .where_eq("weight", Value::Float(1.0))
        .collect_edges()
        .unwrap();
    assert_eq!(edges.len(), 1);

    db.delete_node(a).unwrap();

    let edges = db
        .node(b)
        .in_("knows")
        .where_eq("weight", Value::Float(1.0))
        .collect_edges()
        .unwrap();
    assert_eq!(edges.len(), 0);
}

#[test]
fn test_delete_node_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();

        let a = db.add_node("A", Vec::new()).unwrap();
        let b = db.add_node("B", Vec::new()).unwrap();

        db.add_edge(a, b, "knows", Vec::new()).unwrap();

        db.delete_node(a).unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();

    let result = db.get_node(1);
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(1))));

    let out = db.node(2).in_("knows").collect_edges().unwrap();
    assert!(out.is_empty());
}
