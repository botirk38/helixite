use helixite::HelixiteBuilder;
use helixite::Value;
use helixite::storage::engine::Db;
use helixite::storage::{MemoryStorage, StorageEngine};
use tempfile::tempdir;

#[test]
fn test_vector_key_roundtrip() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let node_id = db
        .add_node(
            "Chunk",
            vec![("embedding".to_string(), Value::Vector(vec![1.0, 2.0, 3.0]))],
        )
        .unwrap();

    let node = db.get_node(node_id).unwrap();
    assert!(matches!(node.properties.get("embedding"), Some(Value::Vector(v)) if v.len() == 3));
}

#[test]
fn test_vector_storage_persists_with_memory() {
    let storage = MemoryStorage::default();
    let db = HelixiteBuilder::default()
        .storage(storage)
        .open(tempdir().unwrap().path())
        .unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![0.5, -0.5, 0.0]))],
    )
    .unwrap();

    let entries = db.storage().scan_prefix(Db::VectorIndexes, &[]).unwrap();
    assert!(!entries.is_empty());
}

#[test]
fn test_multiple_vectors_same_property() {
    let storage = MemoryStorage::default();
    let db = HelixiteBuilder::default()
        .storage(storage)
        .open(tempdir().unwrap().path())
        .unwrap();

    for i in 0..10 {
        db.add_node(
            "Chunk",
            vec![(
                "embedding".to_string(),
                Value::Vector(vec![i as f32, 0.0, 0.0]),
            )],
        )
        .unwrap();
    }

    let entries = db.storage().scan_prefix(Db::VectorIndexes, &[]).unwrap();
    assert_eq!(entries.len(), 10);
}
