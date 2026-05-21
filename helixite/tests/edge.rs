use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_add_edge_returns_id() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db.add_edge(from, to, "knows", Vec::new()).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_get_edge() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.id, id);
    assert_eq!(edge.from, from);
    assert_eq!(edge.to, to);
    assert_eq!(edge.label, "knows");
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2020)));
}

#[test]
fn test_get_missing_edge_returns_error() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.get_edge(999);
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(999))));
}

#[test]
fn test_multiple_edges_get_incrementing_ids() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    let id1 = db.add_edge(a, b, "rel", Vec::new()).unwrap();
    let id2 = db.add_edge(b, c, "rel", Vec::new()).unwrap();
    let id3 = db.add_edge(a, c, "rel", Vec::new()).unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_neighbors_out_direction() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(a).outgoing_any().nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_neighbors_in_direction() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(c).incoming_any().nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_neighbors_with_label_filter() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "follows", Vec::new()).unwrap();

    let knows = db.node(a).outgoing("knows").edges().unwrap();
    assert_eq!(knows.len(), 1);
    assert_eq!(knows[0].label, "knows");

    let follows = db.node(a).outgoing("follows").edges().unwrap();
    assert_eq!(follows.len(), 1);
    assert_eq!(follows[0].label, "follows");
}

#[test]
fn test_edge_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();
        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        db.add_edge(from, to, "knows", Vec::new()).unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();
    let edge = db.get_edge(1).unwrap();
    assert_eq!(edge.label, "knows");
}

#[test]
fn test_mutate_edge_set_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| tx.edge(id).set_property("since", Value::Int(2024)).apply())
        .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2024)));
}

#[test]
fn test_mutate_edge_remove_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| tx.edge(id).remove_property("since").apply())
        .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), None);
}

#[test]
fn test_mutate_edge_replace_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| {
        tx.edge(id)
            .replace_properties(vec![("weight".to_string(), Value::Float(0.8))])
            .apply()
    })
    .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), None);
    assert_eq!(edge.properties.get("weight"), Some(&Value::Float(0.8)));
}

#[test]
fn test_mutate_edge_set_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    let id = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    db.batch(|tx| tx.edge(id).set_label("follows").apply())
        .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.label, "follows");

    let knows = db.node(a).outgoing("knows").edges().unwrap();
    assert!(knows.is_empty());

    let follows = db.node(a).outgoing("follows").edges().unwrap();
    assert_eq!(follows.len(), 1);
}

#[test]
fn test_mutate_edge_label_and_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| {
        tx.edge(id)
            .set_label("friends_with")
            .set_property("since", Value::Int(2024))
            .apply()
    })
    .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.label, "friends_with");
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2024)));
}

#[test]
fn test_mutate_edge_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();
        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        let id = db.add_edge(from, to, "knows", Vec::new()).unwrap();

        db.batch(|tx| {
            tx.edge(id)
                .set_label("follows")
                .set_property("weight", Value::Float(0.5))
                .apply()
        })
        .unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();
    let edge = db.get_edge(1).unwrap();
    assert_eq!(edge.label, "follows");
    assert_eq!(edge.properties.get("weight"), Some(&Value::Float(0.5)));
}

#[test]
fn test_mutate_nonexistent_edge_errors() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.batch(|tx| tx.edge(999).apply());
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(999))));
}

#[test]
fn test_mutate_edge_empty_apply_is_noop() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| tx.edge(id).apply()).unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.label, "knows");
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2020)));
}

#[test]
fn test_mutate_edge_replace_properties_wins_over_set_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| {
        tx.edge(id)
            .set_property("since", Value::Int(2024))
            .replace_properties(vec![("weight".to_string(), Value::Float(0.9))])
            .apply()
    })
    .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), None);
    assert_eq!(edge.properties.get("weight"), Some(&Value::Float(0.9)));
}

#[test]
fn test_mutate_edge_set_property_wins_over_remove_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.batch(|tx| {
        tx.edge(id)
            .remove_property("since")
            .set_property("since", Value::Int(2025))
            .apply()
    })
    .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2025)));
}

#[test]
fn test_mutate_edge_label_remove_property_set_property_combined() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![
                ("since".to_string(), Value::Int(2020)),
                ("weight".to_string(), Value::Float(0.5)),
            ],
        )
        .unwrap();

    db.batch(|tx| {
        tx.edge(id)
            .set_label("friends_with")
            .remove_property("weight")
            .set_property("closeness", Value::Float(0.9))
            .apply()
    })
    .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.label, "friends_with");
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2020)));
    assert_eq!(edge.properties.get("weight"), None);
    assert_eq!(edge.properties.get("closeness"), Some(&Value::Float(0.9)));

    let knows = db.node(from).outgoing("knows").edges().unwrap();
    assert!(knows.is_empty());

    let friends = db.node(from).outgoing("friends_with").edges().unwrap();
    assert_eq!(friends.len(), 1);
    assert_eq!(friends[0].id, id);
}

#[test]
fn test_delete_edge() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db
        .add_edge(
            from,
            to,
            "knows",
            vec![("since".to_string(), Value::Int(2020))],
        )
        .unwrap();

    db.delete_edge(id).unwrap();

    let result = db.get_edge(id);
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(_))));
}

#[test]
fn test_delete_nonexistent_edge_errors() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.delete_edge(999);
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(999))));
}

#[test]
fn test_delete_edge_removes_from_traversal() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    let e1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let e2 = db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let out = db.node(a).outgoing("knows").edges().unwrap();
    assert_eq!(out.len(), 2);

    db.delete_edge(e1).unwrap();

    let out = db.node(a).outgoing("knows").edges().unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, e2);
}

#[test]
fn test_delete_edge_removes_from_in_traversal() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    let e1 = db.add_edge(a, c, "knows", Vec::new()).unwrap();
    let e2 = db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let incoming = db.node(c).incoming("knows").edges().unwrap();
    assert_eq!(incoming.len(), 2);

    db.delete_edge(e1).unwrap();

    let incoming = db.node(c).incoming("knows").edges().unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].id, e2);
}

#[test]
fn test_delete_edge_removes_from_any_traversal() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, b, "follows", Vec::new()).unwrap();

    let count = db.node(a).outgoing_any().count().unwrap();
    assert_eq!(count, 2);

    let edges = db.node(a).outgoing_any().edges().unwrap();
    let first_id = edges[0].id;
    db.delete_edge(first_id).unwrap();

    let count = db.node(a).outgoing_any().count().unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_delete_edge_with_indexed_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    let e1 = db
        .add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let edges = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 1);

    db.delete_edge(e1).unwrap();

    let edges = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 0);
}

#[test]
fn test_delete_edge_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();

        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        let id = db.add_edge(from, to, "knows", Vec::new()).unwrap();

        db.delete_edge(id).unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();

    let result = db.get_edge(1);
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(1))));

    let out = db.node(1).outgoing("knows").edges().unwrap();
    assert!(out.is_empty());
}

#[test]
fn test_get_edges_batch() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    let e1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let e2 = db.add_edge(b, c, "knows", Vec::new()).unwrap();
    let e3 = db.add_edge(a, c, "follows", Vec::new()).unwrap();

    let edges = db.get_edges(&[e1, e2, e3]).unwrap();
    assert_eq!(edges.len(), 3);
    assert_eq!(edges[0].id, e1);
    assert_eq!(edges[1].id, e2);
    assert_eq!(edges[2].id, e3);
}

#[test]
fn test_get_edges_batch_fails_on_missing() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let e1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.get_edges(&[e1, 999, 1000]);

    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(999))));
}

#[test]
fn test_edge_label_index_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, b, "follows", Vec::new()).unwrap();

    drop(db);

    let db = HelixiteBuilder::new().open(dir.path()).unwrap();
    let knows = db.edges().label("knows").collect().unwrap();
    assert_eq!(knows.len(), 1);
    assert_eq!(knows[0].label, "knows");
}

#[test]
fn test_delete_edge_removes_from_label_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    let edge = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    db.delete_edge(edge).unwrap();

    let knows = db.edges().label("knows").collect().unwrap();
    assert!(knows.is_empty());
}

#[test]
fn test_mutate_edge_label_updates_label_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    let edge = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    db.update_edge(edge).set_label("follows").apply().unwrap();

    let knows = db.edges().label("knows").collect().unwrap();
    assert!(knows.is_empty());

    let follows = db.edges().label("follows").collect().unwrap();
    assert_eq!(follows.len(), 1);
    assert_eq!(follows[0].id, edge);
}

#[test]
fn test_delete_node_cascades_to_edge_label_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    db.delete_node(a).unwrap();

    let knows = db.edges().label("knows").collect().unwrap();
    assert!(knows.is_empty());
}
