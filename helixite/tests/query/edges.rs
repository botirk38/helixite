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

#[test]
fn test_edges_collect_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.edges().label("knows").after("n:1").collect();
    assert!(matches!(result, Err(HelixiteError::InvalidConfig(_))));
}

#[test]
fn test_edges_ids_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.edges().label("knows").after("n:1").ids();
    assert!(matches!(result, Err(HelixiteError::InvalidConfig(_))));
}

#[test]
fn test_edges_count_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.edges().label("knows").after("n:1").count();
    assert!(matches!(result, Err(HelixiteError::InvalidConfig(_))));
}

#[test]
fn test_edges_first_rejects_after() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.edges().label("knows").after("n:1").first();
    assert!(matches!(result, Err(HelixiteError::InvalidConfig(_))));
}

#[test]
fn test_edges_page_zero_size_rejected() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.edges().label("knows").page(0);
    assert!(matches!(result, Err(HelixiteError::InvalidConfig(_))));
}

#[test]
fn test_edges_limit_conflicts_with_page() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.edges().label("knows").limit(1).page(1);
    assert!(matches!(result, Err(HelixiteError::InvalidConfig(_))));
}

#[test]
fn test_edges_invalid_cursor() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    db.add_edge(a, b, "knows", Vec::new()).unwrap();

    let result = db.edges().label("knows").after("bad-cursor").page(1);
    assert!(matches!(result, Err(HelixiteError::InvalidCursor(_))));
}

#[test]
fn test_edges_stale_cursor() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();
    let edge = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    db.delete_edge(edge).unwrap();

    let cursor = format!("e:{}", edge);
    let result = db.edges().label("knows").after(&cursor).page(1);
    assert!(matches!(result, Err(HelixiteError::InvalidCursor(_))));
}

#[test]
fn test_edges_page_with_property_filter() {
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

    let first = db
        .edges()
        .label("knows")
        .eq("since", Value::Int(2020))
        .page(1)
        .unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].properties.get("since"), Some(&Value::Int(2020)));
    assert!(first.next_cursor.is_none());

    let empty = db
        .edges()
        .label("knows")
        .eq("since", Value::Int(2020))
        .after("dummy")
        .page(1);
    assert!(matches!(empty, Err(HelixiteError::InvalidCursor(_))));
}

#[test]
fn test_edges_mixed_labels_collect() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    let edge1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let edge2 = db.add_edge(a, b, "follows", Vec::new()).unwrap();
    let edge3 = db.add_edge(a, b, "blocks", Vec::new()).unwrap();

    let all = db.edges().collect().unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].id, edge1);
    assert_eq!(all[1].id, edge2);
    assert_eq!(all[2].id, edge3);
}

#[test]
fn test_edges_mixed_labels_ids() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    let edge1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let edge2 = db.add_edge(a, b, "follows", Vec::new()).unwrap();
    let edge3 = db.add_edge(a, b, "blocks", Vec::new()).unwrap();

    let ids = db.edges().ids().unwrap();
    assert_eq!(ids, vec![edge1, edge2, edge3]);
}

#[test]
fn test_edges_page_without_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let a = db.add_node("User", Vec::new()).unwrap();
    let b = db.add_node("User", Vec::new()).unwrap();

    let edge1 = db.add_edge(a, b, "knows", Vec::new()).unwrap();
    let edge2 = db.add_edge(a, b, "follows", Vec::new()).unwrap();

    let first = db.edges().page(1).unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].id, edge1);
    assert!(first.next_cursor.is_some());

    let second = db
        .edges()
        .after(first.next_cursor.unwrap())
        .page(1)
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].id, edge2);
    assert!(second.next_cursor.is_none());
}
