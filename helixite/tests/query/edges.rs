use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_edges_by_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.add_edge(a, b, "follows", Vec::new()).unwrap();

    let knows = db.edges().label("knows").collect().unwrap();
    assert_eq!(knows.len(), 1);
    assert_eq!(knows[0].label, "knows");
}

#[test]
fn test_edges_by_property() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    let c = db.add_node("User", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("since".to_string(), Value::Int(2020))])
        .unwrap();
    db.add_edge(a, c, "knows", [("since".to_string(), Value::Int(2024))])
        .unwrap();

    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    let edges = db
        .edges()
        .label("knows")
        .eq("since", Value::Int(2024))
        .collect()
        .unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to, c);
}

#[test]
fn test_edges_limit_count_ids_and_first() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    let edge1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let edge2 = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    assert_eq!(db.edges().label("knows").count().unwrap(), 2);
    assert_eq!(
        db.edges().label("knows").limit(1).collect().unwrap().len(),
        1
    );
    assert_eq!(db.edges().label("knows").ids().unwrap(), vec![edge1, edge2]);
    assert_eq!(
        db.edges().label("knows").first().unwrap().unwrap().id,
        edge1
    );
}

#[test]
fn test_edges_page() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    let edge1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let edge2 = db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let first = db.edges().label("knows").page(1).unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].id, edge1);
    assert!(first.next_cursor.is_some());

    let second = db
        .edges()
        .label("knows")
        .after(first.next_cursor.unwrap())
        .page(1)
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].id, edge2);
    assert!(second.next_cursor.is_none());
}

#[test]
fn test_edges_property_filter_requires_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    db.add_edge(a, b, "knows", [("since".to_string(), Value::Int(2020))])
        .unwrap();

    let result = db
        .edges()
        .label("knows")
        .eq("since", Value::Int(2020))
        .collect();

    assert!(matches!(result, Err(HelixiteError::IndexNotFound(_))));
}

#[test]
fn test_edges_property_filter_requires_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.edges().eq("since", Value::Int(2020)).collect();

    assert!(matches!(result, Err(HelixiteError::IndexNotFound(_))));
}
