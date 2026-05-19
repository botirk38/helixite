use helixite::storage::{Db, MemoryStorage, StorageEngine};

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

    let results = storage
        .read(|txn| txn.scan_prefix(Db::Nodes, &[1]))
        .unwrap();
    assert_eq!(results.len(), 2);
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
