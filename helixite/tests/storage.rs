use helixite::storage::{Db, StorageEngine, StorageTxn};
use helixite::Helixite;
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::tempdir;

type MemoryKey = (Db, Vec<u8>);

#[derive(Default)]
struct MemoryStorage {
    data: Mutex<HashMap<MemoryKey, Vec<u8>>>,
}

struct MemoryTxn<'a> {
    data: &'a Mutex<HashMap<MemoryKey, Vec<u8>>>,
    snapshot: HashMap<MemoryKey, Vec<u8>>,
    committed: bool,
}

impl StorageTxn for MemoryTxn<'_> {
    fn get(&self, db: Db, key: &[u8]) -> helixite::Result<Option<Vec<u8>>> {
        Ok(self.snapshot.get(&(db, key.to_vec())).cloned())
    }

    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> helixite::Result<()> {
        self.snapshot
            .insert((db, key.to_vec()), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, db: Db, key: &[u8]) -> helixite::Result<()> {
        self.snapshot.remove(&(db, key.to_vec()));
        Ok(())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> helixite::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .snapshot
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }
}

impl Drop for MemoryTxn<'_> {
    fn drop(&mut self) {
        if self.committed {
            let mut data = self.data.lock().unwrap();
            for (key, value) in self.snapshot.drain() {
                data.insert(key, value);
            }
        }
    }
}

impl StorageEngine for MemoryStorage {
    fn get(&self, db: Db, key: &[u8]) -> helixite::Result<Option<Vec<u8>>> {
        Ok(self.data.lock().unwrap().get(&(db, key.to_vec())).cloned())
    }

    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> helixite::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .iter()
            .filter(|((stored_db, key), _)| *stored_db == db && key.starts_with(prefix))
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect())
    }

    fn write<F, T>(&self, f: F) -> helixite::Result<T>
    where
        F: FnOnce(&mut dyn StorageTxn) -> helixite::Result<T>,
    {
        let snapshot = self.data.lock().unwrap().clone();
        let mut txn = MemoryTxn {
            data: &self.data,
            snapshot,
            committed: false,
        };
        let result = f(&mut txn);
        if result.is_ok() {
            txn.committed = true;
        }
        result
    }
}

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

    let result = storage.get(Db::Nodes, &[1]).unwrap();
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

    let result = storage.get(Db::Nodes, &[1]).unwrap();
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

    let results = storage.scan_prefix(Db::Nodes, &[1]).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_memory_txn_write_aborts_on_error() {
    let storage = MemoryStorage::default();
    let _ = storage.write(|txn| {
        txn.put(Db::Nodes, &[1], b"should_not_exist")?;
        Err::<(), _>(helixite::HelixiteError::Storage("abort".into()))
    });

    let result = storage.get(Db::Nodes, &[1]).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_lmdb_storage_get_put_scan_delete() {
    let dir = tempdir().unwrap();
    let db = Helixite::open(dir.path(), None).unwrap();

    db.storage()
        .write(|txn| {
            txn.put(Db::Metadata, b"key1", b"value1")?;
            txn.put(Db::Metadata, b"key2", b"value2")?;
            txn.put(Db::Nodes, b"node1", b"nodedata")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let val = db.storage().get(Db::Metadata, b"key1").unwrap();
    assert_eq!(val, Some(b"value1".to_vec()));

    let missing = db.storage().get(Db::Metadata, b"missing").unwrap();
    assert_eq!(missing, None);

    let prefix_results = db.storage().scan_prefix(Db::Metadata, b"key").unwrap();
    assert_eq!(prefix_results.len(), 2);

    db.storage()
        .write(|txn| {
            txn.delete(Db::Metadata, b"key1")?;
            Ok::<_, helixite::HelixiteError>(())
        })
        .unwrap();

    let deleted = db.storage().get(Db::Metadata, b"key1").unwrap();
    assert_eq!(deleted, None);
}
