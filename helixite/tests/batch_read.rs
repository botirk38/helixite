use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_get_nodes_batch() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id1 = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".into()))],
        )
        .unwrap();
    let id2 = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Bob".into()))],
        )
        .unwrap();
    let id3 = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Carol".into()))],
        )
        .unwrap();

    let nodes = db.get_nodes(&[id1, id2, id3]).unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].id, id1);
    assert_eq!(nodes[1].id, id2);
    assert_eq!(nodes[2].id, id3);
}

#[test]
fn test_get_nodes_batch_skips_missing() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id1 = db.add_node("User", Vec::new()).unwrap();

    let nodes = db.get_nodes(&[id1, 999, 1000]).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, id1);
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
fn test_get_edges_batch_skips_missing() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let e1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let edges = db.get_edges(&[e1, 999, 1000]).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].id, e1);
}

#[test]
fn test_read_txn_snapshot_consistency() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".into()))],
        )
        .unwrap();

    let snapshot_name = db
        .read(|tx| {
            let node = tx.get_node(id)?;
            let updated = db.add_node(
                "User",
                vec![("name".to_string(), Value::String("Bob".into()))],
            );
            assert!(updated.is_ok());
            let node2 = tx.get_node(id)?;
            Ok((node, node2))
        })
        .unwrap();

    assert_eq!(
        snapshot_name.0.properties.get("name"),
        snapshot_name.1.properties.get("name")
    );
}

#[test]
fn test_read_closure_returns_error() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.read(|tx| tx.get_node(999));
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_read_txn_multiple_ops() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id1 = db.add_node("User", Vec::new()).unwrap();
    let id2 = db.add_node("User", Vec::new()).unwrap();
    let id3 = db.add_node("User", Vec::new()).unwrap();

    let result = db
        .read(|tx| {
            let n1 = tx.get_node(id1)?;
            let n2 = tx.get_node(id2)?;
            let n3 = tx.get_node(id3)?;
            Ok((n1.id, n2.id, n3.id))
        })
        .unwrap();

    assert_eq!(result, (id1, id2, id3));
}
