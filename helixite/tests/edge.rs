use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_add_edge_returns_id() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let from = db.add_node("User", Vec::new()).unwrap();
    let to = db.add_node("User", Vec::new()).unwrap();

    let id = db.add_edge(from, to, "knows", Vec::new()).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_get_edge() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db.get_edge(999);
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(999))));
}

#[test]
fn test_multiple_edges_get_incrementing_ids() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(a).out_any().collect_nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_neighbors_in_direction() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, c, "knows", Vec::new()).unwrap();
    db.add_edge(b, c, "knows", Vec::new()).unwrap();

    let neighbors = db.node(c).in_any().collect_nodes().unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_neighbors_with_label_filter() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();
    let c = db.add_node("C", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, c, "follows", Vec::new()).unwrap();

    let knows = db.node(a).out("knows").collect_edges().unwrap();
    assert_eq!(knows.len(), 1);
    assert_eq!(knows[0].label, "knows");

    let follows = db.node(a).out("follows").collect_edges().unwrap();
    assert_eq!(follows.len(), 1);
    assert_eq!(follows[0].label, "follows");
}

#[test]
fn test_edge_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        db.add_edge(from, to, "knows", Vec::new()).unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let edge = db.get_edge(1).unwrap();
    assert_eq!(edge.label, "knows");
}

#[test]
fn test_mutate_edge_set_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    db.edge_mut(id)
        .set_property("since", Value::Int(2024))
        .apply()
        .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), Some(&Value::Int(2024)));
}

#[test]
fn test_mutate_edge_remove_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    db.edge_mut(id).remove_property("since").apply().unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), None);
}

#[test]
fn test_mutate_edge_replace_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    db.edge_mut(id)
        .replace_properties(vec![("weight".to_string(), Value::Float(0.8))])
        .apply()
        .unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.properties.get("since"), None);
    assert_eq!(edge.properties.get("weight"), Some(&Value::Float(0.8)));
}

#[test]
fn test_mutate_edge_set_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let a = db.add_node("A", Vec::new()).unwrap();
    let b = db.add_node("B", Vec::new()).unwrap();

    let id = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    db.edge_mut(id).set_label("follows").apply().unwrap();

    let edge = db.get_edge(id).unwrap();
    assert_eq!(edge.label, "follows");

    let knows = db.node(a).out("knows").collect_edges().unwrap();
    assert!(knows.is_empty());

    let follows = db.node(a).out("follows").collect_edges().unwrap();
    assert_eq!(follows.len(), 1);
}

#[test]
fn test_mutate_edge_label_and_properties() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

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

    db.edge_mut(id)
        .set_label("friends_with")
        .set_property("since", Value::Int(2024))
        .apply()
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
        let db = HelixiteBuilder::default().open(path).unwrap();
        let from = db.add_node("User", Vec::new()).unwrap();
        let to = db.add_node("User", Vec::new()).unwrap();
        let id = db.add_edge(from, to, "knows", Vec::new()).unwrap();

        db.edge_mut(id)
            .set_label("follows")
            .set_property("weight", Value::Float(0.5))
            .apply()
            .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let edge = db.get_edge(1).unwrap();
    assert_eq!(edge.label, "follows");
    assert_eq!(edge.properties.get("weight"), Some(&Value::Float(0.5)));
}

#[test]
fn test_mutate_nonexistent_edge_errors() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let result = db.edge_mut(999).apply();
    assert!(matches!(result, Err(HelixiteError::EdgeNotFound(999))));
}
