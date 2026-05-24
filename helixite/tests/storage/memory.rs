use helixite::storage::{Db, MemoryStorage, Scan, StorageEngine};

#[test]
fn test_db_variants_exist() {
    let dbs = [
        Db::Metadata,
        Db::Nodes,
        Db::Edges,
        Db::OutEdges,
        Db::InEdges,
        Db::Labels,
        Db::Properties,
        Db::VectorIndexes,
    ];
    assert_eq!(dbs.len(), 8);
}

#[test]
fn test_db_equality() {
    assert_eq!(Db::Metadata, Db::Metadata);
    assert_ne!(Db::Metadata, Db::Nodes);
}

#[test]
fn test_db_hashable() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Db::Metadata);
    set.insert(Db::Nodes);
    set.insert(Db::Edges);
    assert_eq!(set.len(), 3);
}

#[test]
fn test_memory_txn_get_put() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"node_data")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let result = storage.read(|txn| txn.get(Db::Nodes, &[1])).unwrap();
    assert_eq!(result, Some(b"node_data".to_vec()));
}

#[test]
fn test_memory_txn_delete() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"node_data")?;
            txn.delete(Db::Nodes, &[1])?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let result = storage.read(|txn| txn.get(Db::Nodes, &[1])).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_memory_txn_delete_persists_across_transactions() {
    let storage = MemoryStorage::default();

    storage
        .write(|txn| {
            txn.put(Db::Metadata, b"key", b"value")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    assert_eq!(
        storage.read(|txn| txn.get(Db::Metadata, b"key")).unwrap(),
        Some(b"value".to_vec())
    );

    storage
        .write(|txn| {
            txn.delete(Db::Metadata, b"key")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let result = storage
        .read(|txn| txn.get(Db::Metadata, b"key"))
        .unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_memory_txn_scan_prefix() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1, 0], b"node_0")?;
            txn.put(Db::Nodes, &[1, 1], b"node_1")?;
            txn.put(Db::Nodes, &[2, 0], b"other")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    storage
        .read(|txn| {
            let results = txn.scan(Db::Nodes, Scan::Prefix(&[1]), None)?;
            assert_eq!(results.len(), 2);
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
}

#[test]
fn test_memory_txn_write_aborts_on_error() {
    let storage = MemoryStorage::default();
    let _ = storage.write(|txn| {
        txn.put(Db::Nodes, &[1], b"should_not_exist")?;
        Err::<(), _>(helixite::HelixiteError::Storage("abort".into()))
    });

    let result = storage.read(|txn| txn.get(Db::Nodes, &[1])).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_memory_scan_all() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"a")?;
            txn.put(Db::Nodes, &[2], b"b")?;
            txn.put(Db::Nodes, &[3], b"c")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    storage
        .read(|txn| {
            let results = txn.scan(Db::Nodes, Scan::All, None)?;
            assert_eq!(results.len(), 3);
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
}

#[test]
fn test_memory_scan_prefix_no_match() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1, 0], b"data")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    storage
        .read(|txn| {
            let results = txn.scan(Db::Nodes, Scan::Prefix(&[99]), None)?;
            assert!(results.is_empty());
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
}

#[test]
fn test_memory_scan_with_limit() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            for i in 0..10u8 {
                txn.put(Db::Nodes, &[i], &[i])?;
            }
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    storage
        .read(|txn| {
            let results = txn.scan(Db::Nodes, Scan::All, Some(3))?;
            assert_eq!(results.len(), 3);
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
}

#[test]
fn test_memory_put_overwrite() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"first")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"second")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let result = storage.read(|txn| txn.get(Db::Nodes, &[1])).unwrap();
    assert_eq!(result, Some(b"second".to_vec()));
}

#[test]
fn test_memory_delete_nonexistent_key() {
    let storage = MemoryStorage::default();
    let result = storage.write(|txn| {
        txn.delete(Db::Nodes, &[99])?;
        Ok::<_, helixite::HelixiteError>(())
    });
    assert!(result.is_ok());
}

#[test]
fn test_memory_cross_db_isolation() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"node")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let result = storage.read(|txn| txn.get(Db::Edges, &[1])).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_memory_get_missing_key() {
    let storage = MemoryStorage::default();
    let result = storage.read(|txn| txn.get(Db::Nodes, &[42])).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_memory_close() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"data")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
    assert!(storage.close().is_ok());
}

#[test]
fn test_memory_iter_all_returns_all_entries() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1], b"a")?;
            txn.put(Db::Nodes, &[2], b"b")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    storage
        .read(|txn| {
            let entries: Vec<_> = txn.iter(Db::Nodes, Scan::All)?.collect::<Result<_, _>>()?;
            assert_eq!(entries.len(), 2);
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
}

#[test]
fn test_memory_scan_prefix_with_limit() {
    let storage = MemoryStorage::default();
    storage
        .write(|txn| {
            txn.put(Db::Nodes, &[1, 0], b"a")?;
            txn.put(Db::Nodes, &[1, 1], b"b")?;
            txn.put(Db::Nodes, &[1, 2], b"c")?;
            txn.put(Db::Nodes, &[2, 0], b"other")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    storage
        .read(|txn| {
            let results = txn.scan(Db::Nodes, Scan::Prefix(&[1]), Some(2))?;
            assert_eq!(results.len(), 2);
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();
}
