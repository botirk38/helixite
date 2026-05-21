use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_read_txn_snapshot_consistency() {
    use helixite::storage::memory::MemoryStorage;

    let storage = MemoryStorage::new();
    let db = helixite::HelixiteBuilder::new()
        .storage(storage)
        .open(tempdir().unwrap().path())
        .unwrap();

    let id = db
        .add_node(
            "User",
            vec![("name".to_string(), Value::String("Alice".into()))],
        )
        .unwrap();

    let old_value = db
        .read(|tx| {
            let node = tx.get_node(id)?;
            let name = node.properties.get("name").cloned();

            db.update_node(id)
                .set_property("name", Value::String("Bob".into()))
                .apply()
                .unwrap();

            let node2 = tx.get_node(id)?;
            let name2 = node2.properties.get("name").cloned();

            Ok((name, name2))
        })
        .unwrap();

    assert_eq!(old_value.0, Some(Value::String("Alice".into())));
    assert_eq!(old_value.1, Some(Value::String("Alice".into())));

    let fresh = db.get_node(id).unwrap();
    assert_eq!(
        fresh.properties.get("name"),
        Some(&Value::String("Bob".into()))
    );
}

#[test]
fn test_read_closure_returns_error() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.read(|tx| tx.get_node(999));
    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
}

#[test]
fn test_read_txn_multiple_ops() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id1 = db.add_node("User", Vec::new()).unwrap();
    let id2 = db.add_node("User", Vec::new()).unwrap();
    let id3 = db.add_node("User", Vec::new()).unwrap();

    let result = db
        .read(|tx| {
            let n1 = tx.get_node(id1)?;
            let n2 = tx.get_node(id2)?;
            let n3 = tx.get_node(id3)?;
            Ok((n1.id, n2.id, n3.id))
        })
        .unwrap();

    assert_eq!(result, (id1, id2, id3));
}

#[test]
fn test_batch_write_multiple_ops() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let (a, b, edge) = db
        .batch(|tx| {
            let a = tx.add_node(
                "User",
                [("name".to_string(), Value::String("Alice".into()))],
            )?;
            let b = tx.add_node("User", [("name".to_string(), Value::String("Bob".into()))])?;
            let edge = tx.add_edge(a, b, "knows", [("since".to_string(), Value::Int(2024))])?;
            Ok((a, b, edge))
        })
        .unwrap();

    assert_eq!(db.get_node(a).unwrap().label, "User");
    assert_eq!(db.get_node(b).unwrap().label, "User");
    assert_eq!(db.get_edge(edge).unwrap().from, a);
}

#[test]
fn test_batch_rolls_back_on_error() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.batch(|tx| {
        tx.add_node(
            "User",
            [("name".to_string(), Value::String("Alice".into()))],
        )?;
        tx.get_node(999)?;
        Ok(())
    });

    assert!(matches!(result, Err(HelixiteError::NodeNotFound(999))));
    assert_eq!(db.nodes().label("User").count().unwrap(), 0);
}
