use ivy::IvyBuilder;
use ivy::storage::{Db, Scan, StorageEngine};
use tempfile::tempdir;

#[test]
fn test_lmdb_storage_get_put_scan_delete() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"key1", b"value1")?;
            txn.put(Db::Metadata, b"key2", b"value2")?;
            txn.put(Db::Nodes, b"node1", b"nodedata")?;
            Ok::<_, ivy::IvyError>(())
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

    db.storage()
        .read(|txn| {
            let prefix_results = txn.scan(Db::Metadata, Scan::Prefix(b"key"), None)?;
            assert_eq!(prefix_results.len(), 2);
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .write(|txn| {
            txn.delete(Db::Metadata, b"key1")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    let deleted = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"key1"))
        .unwrap();
    assert_eq!(deleted, None);
}

#[test]
fn test_lmdb_scan_all() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"a", b"1")?;
            txn.put(Db::Metadata, b"b", b"2")?;
            txn.put(Db::Metadata, b"c", b"3")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .read(|txn| {
            let results = txn.scan(Db::Metadata, Scan::All, None)?;
            assert!(results.len() >= 3);
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();
}

#[test]
fn test_lmdb_scan_prefix_no_match() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"key1", b"val")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .read(|txn| {
            let results = txn.scan(Db::Metadata, Scan::Prefix(b"zzz"), None)?;
            assert!(results.is_empty());
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();
}

#[test]
fn test_lmdb_scan_with_limit() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            for i in 0..10u8 {
                txn.put(Db::Metadata, &[b'k', i], &[i])?;
            }
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .read(|txn| {
            let results = txn.scan(Db::Metadata, Scan::Prefix(b"k"), Some(3))?;
            assert_eq!(results.len(), 3);
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();
}

#[test]
fn test_lmdb_put_overwrite() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"key", b"first")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"key", b"second")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    let result = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"key"))
        .unwrap();
    assert_eq!(result, Some(b"second".to_vec()));
}

#[test]
fn test_lmdb_delete_nonexistent_key() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let result = db.storage().write(|txn| {
        txn.delete(Db::Metadata, b"missing")?;
        Ok::<_, ivy::IvyError>(())
    });
    assert!(result.is_ok());
}

#[test]
fn test_lmdb_cross_db_isolation() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Nodes, b"key", b"node_data")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    let result = db.storage().read(|txn| txn.get(Db::Edges, b"key")).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_lmdb_write_abort_on_error() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let _ = db.storage().write(|txn| {
        txn.put(Db::Metadata, b"key", b"should_not_exist")?;
        Err::<(), _>(ivy::IvyError::Storage("abort".into()))
    });

    let result = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"key"))
        .unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_lmdb_get_missing_key() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    let result = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"nonexistent"))
        .unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_lmdb_iter_all() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"x", b"1")?;
            txn.put(Db::Metadata, b"y", b"2")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .read(|txn| {
            let entries: Vec<_> = txn
                .iter(Db::Metadata, Scan::All)?
                .collect::<Result<_, _>>()?;
            assert!(entries.len() >= 2);
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();
}

#[test]
fn test_lmdb_scan_prefix_with_limit() {
    let dir = tempdir().unwrap();
    let db = IvyBuilder::new().open(dir.path()).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"pfx_a", b"1")?;
            txn.put(Db::Metadata, b"pfx_b", b"2")?;
            txn.put(Db::Metadata, b"pfx_c", b"3")?;
            txn.put(Db::Metadata, b"other", b"4")?;
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();

    db.storage()
        .read(|txn| {
            let results = txn.scan(Db::Metadata, Scan::Prefix(b"pfx"), Some(2))?;
            assert_eq!(results.len(), 2);
            Ok::<_, ivy::IvyError>(())
        })
        .unwrap();
}

#[test]
fn test_lmdb_data_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = IvyBuilder::new().open(path).unwrap();
        db.storage()
            .write(|txn| {
                txn.put(Db::Metadata, b"persist_key", b"persist_val")?;
                Ok::<_, ivy::IvyError>(())
            })
            .unwrap();
    }

    let db = IvyBuilder::new().open(path).unwrap();
    let result = db
        .storage()
        .read(|txn| txn.get(Db::Metadata, b"persist_key"))
        .unwrap();
    assert_eq!(result, Some(b"persist_val".to_vec()));
}
