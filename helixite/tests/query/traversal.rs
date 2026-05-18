use helixite::HelixiteBuilder;
use tempfile::tempdir;

#[test]
fn test_node_traversal_out() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(a).out("knows").collect_nodes().unwrap();
    assert_eq!(neighbors.len(), 2);

    let edges = db.node(a).out("knows").collect_edges().unwrap();
    assert_eq!(edges.len(), 2);
}

#[test]
fn test_node_traversal_in() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(c).in_("knows").collect_nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_node_traversal_any() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, b, "follows", Vec::new()).unwrap();

    let out_count = db.node(a).out_any().count().unwrap();
    assert_eq!(out_count, 2);
}

#[test]
fn test_traversal_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        let a = db.add_node("A", Vec::new()).unwrap();
        let b = db.add_node("B", Vec::new()).unwrap();
        db.add_edge(a, b, "knows", Vec::new()).unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let neighbors = db.node(1).out("knows").collect_nodes().unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].id, 2);
}
