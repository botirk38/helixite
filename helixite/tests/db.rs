use helixite::{Config, HelixiteBuilder, storage::MemoryStorage, value::Value};
use tempfile::tempdir;

#[test]
fn test_open_new_db() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();
    assert!(db.path().exists());
}

#[test]
fn test_reopen_db() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let db1 = HelixiteBuilder::new().open(path).unwrap();
    drop(db1);

    let db2 = HelixiteBuilder::new().open(path).unwrap();
    assert!(db2.path().exists());
}

#[test]
fn test_close_syncs_and_consumes_db() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let db = HelixiteBuilder::new().open(path).unwrap();
    let node = db.add_node("User", Vec::new()).unwrap();

    db.close().unwrap();

    let db = HelixiteBuilder::new().open(path).unwrap();
    assert_eq!(db.get_node(node).unwrap().id, node);
}

#[test]
fn test_open_with_config() {
    let dir = tempdir().unwrap();
    let config = Config::default().with_map_size(64 * 1024 * 1024);
    let db = HelixiteBuilder::new()
        .config(config)
        .open(dir.path())
        .unwrap();
    assert!(db.path().exists());
}

#[test]
fn test_open_with_storage_accepts_custom_engine() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new()
        .storage(MemoryStorage::default())
        .open(dir.path())
        .unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();
    let node = db.get_node(id).unwrap();

    assert_eq!(node.label, "User");
}

#[test]
fn test_graph_stats_counts_labels_and_indexes() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let alice = db
        .add_node("User", [("email".to_string(), Value::String("a@x".into()))])
        .unwrap();
    let bob = db
        .add_node("User", [("email".to_string(), Value::String("b@x".into()))])
        .unwrap();
    db.add_node("Org", Vec::new()).unwrap();
    db.add_edge(
        alice,
        bob,
        "knows",
        [("since".to_string(), Value::Int(2020))],
    )
    .unwrap();

    db.indexes()
        .nodes()
        .create_property("User", "email")
        .unwrap();
    db.indexes()
        .edges()
        .create_property("knows", "since")
        .unwrap();

    let stats = db.stats().unwrap();

    assert_eq!(stats.node_count, 3);
    assert_eq!(stats.edge_count, 1);
    assert_eq!(
        stats
            .labels
            .iter()
            .find(|label| label.label == "User")
            .unwrap()
            .node_count,
        2
    );
    assert_eq!(
        stats.indexes.node_properties.get("User").unwrap(),
        &vec!["email".to_string()]
    );
    assert_eq!(
        stats.indexes.edge_properties.get("knows").unwrap(),
        &vec!["since".to_string()]
    );
}
