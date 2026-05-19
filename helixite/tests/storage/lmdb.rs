use helixite::HelixiteBuilder;
use helixite::storage::{Db, StorageEngine};
use tempfile::tempdir;

#[test]
fn test_lmdb_storage_get_put_scan_delete() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"key1", b"value1")?;
            txn.put(Db::Metadata, b"key2", b"value2")?;
            txn.put(Db::Nodes, b"node1", b"nodedata")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let val = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"key1"))
        .unwrap();
    assert_eq!(val, Some(b"value1".to_vec()));

    let missing = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"missing"))
        .unwrap();
    assert_eq!(missing, None);

    let prefix_results = db
        .storage()
        .read(|txn| txn.scan_prefix(Db::Metadata, b"key"))
        .unwrap();
    assert_eq!(prefix_results.len(), 2);

    db.storage()
        .write(|txn| {
            txn.delete(Db::Metadata, b"key1")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let deleted = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"key1"))
        .unwrap();
    assert_eq!(deleted, None);
}
