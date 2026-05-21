use helixite::{Config, HelixiteBuilder, storage::MemoryStorage};
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
