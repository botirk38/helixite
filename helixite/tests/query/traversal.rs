use helixite::HelixiteBuilder;
use helixite::value::Value;
use tempfile::tempdir;

#[test]
fn test_node_traversal_out() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(a).outgoing("knows").nodes().unwrap();
    assert_eq!(neighbors.len(), 2);

    let edges = db.node(a).outgoing("knows").edges().unwrap();
    assert_eq!(edges.len(), 2);
}

#[test]
fn test_node_traversal_in() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(c).incoming("knows").nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_node_traversal_any() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, b, "follows", Vec::new()).unwrap();

    let out_count = db.node(a).outgoing_any().count().unwrap();
    assert_eq!(out_count, 2);
}

#[test]
fn test_traversal_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::new().open(path).unwrap();
        let a = db.add_node("A", Vec::new()).unwrap();
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let db = HelixiteBuilder::new().open(path).unwrap();
    let neighbors = db.node(1).outgoing("knows").nodes().unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].id, 2);
}

#[test]
fn test_traversal_where_eq_indexed() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(1.0))])
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
    assert_eq!(edges.len(), 2);

    let nodes = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(2.0))
        .nodes()
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, c);

    let count = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .count()
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_traversal_where_eq_not_indexed() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();

    let result = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges();
    assert!(result.is_err());
}

#[test]
fn test_traversal_where_eq_no_label() {
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

    let result = db
        .node(a)
        .outgoing_any()
        .eq("weight", Value::Float(1.0))
        .edges();
    assert!(result.is_err());
}

#[test]
fn test_traversal_where_eq_multiple_filters() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(
        a,
        b,
        "knows",
        [
            ("weight".to_string(), Value::Float(1.0)),
            ("status".to_string(), Value::String("active".to_string())),
        ],
    )
    .unwrap();
    db.add_edge(
        a,
        c,
        "knows",
        [
            ("weight".to_string(), Value::Float(1.0)),
            ("status".to_string(), Value::String("inactive".to_string())),
        ],
    )
    .unwrap();
    db.add_edge(
        a,
        c,
        "knows",
        [
            ("weight".to_string(), Value::Float(2.0)),
            ("status".to_string(), Value::String("active".to_string())),
        ],
    )
    .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();
    db.indexes()
        .edges()
        .create_property("knows", "status")
        .unwrap();

    let edges = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .eq("status", Value::String("active".to_string()))
        .edges()
        .unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to, b);
}

#[test]
fn test_traversal_where_eq_in_direction() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(b, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let nodes = db
        .node(c)
        .incoming("knows")
        .eq("weight", Value::Float(1.0))
        .nodes()
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, a);
}

#[test]
fn test_traversal_reflects_edge_mutation() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("weight".to_string(), Value::Float(1.0))])
        .unwrap();
    db.add_edge(a, c, "knows", [("weight".to_string(), Value::Float(2.0))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "weight")
        .unwrap();

    let edges_before = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges_before.len(), 1);

    let edge_id = edges_before[0].id;
    db.write(|tx| {
        tx.edge(edge_id)
            .set_property("weight", Value::Float(3.0))
            .apply()
    })
    .unwrap();

    let edges_after_old = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(1.0))
        .edges()
        .unwrap();
    assert_eq!(edges_after_old.len(), 0);

    let edges_after_new = db
        .node(a)
        .outgoing("knows")
        .eq("weight", Value::Float(3.0))
        .edges()
        .unwrap();
    assert_eq!(edges_after_new.len(), 1);
}
