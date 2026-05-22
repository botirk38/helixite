use helixite::{HelixiteBuilder, HelixiteError, Value};
use tempfile::tempdir;

#[test]
fn test_write_txn_reads_own_writes() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db
        .batch(|tx| {
            let id = tx.add_node(
                "User",
                [("name".to_string(), Value::String("Alice".into()))],
            )?;
            let node = tx.get_node(id)?;
            Ok(node)
        })
        .unwrap();

    assert_eq!(result.label, "User");
    assert_eq!(
        result.properties.get("name"),
        Some(&Value::String("Alice".into()))
    );
}

#[test]
fn test_batch_multiple_node_edge_mutations() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.batch(|tx| {
        let a = tx.add_node(
            "User",
            [("name".to_string(), Value::String("Alice".into()))],
        )?;
        let b = tx.add_node("User", [("name".to_string(), Value::String("Bob".into()))])?;
        let edge = tx.add_edge(a, b, "knows", [("since".to_string(), Value::Int(2024))])?;

        tx.node(a).set_property("age", Value::Int(30)).apply()?;
        tx.edge(edge)
            .set_property("weight", Value::Float(0.5))
            .apply()?;
        tx.delete_node(b)?;
        Ok(())
    })
    .unwrap();

    let stats = db.stats().unwrap();
    assert_eq!(stats.node_count, 1);
    assert_eq!(stats.edge_count, 0);

    let node = db.get_node(1).unwrap();
    assert_eq!(node.properties.get("age"), Some(&Value::Int(30)));
}

#[test]
fn test_batch_error_midway_rolls_back_all() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let result = db.batch(|tx| {
        tx.add_node("User", [("name".to_string(), Value::String("A".into()))])?;
        tx.add_node("User", [("name".to_string(), Value::String("B".into()))])?;
        tx.add_node("User", [("name".to_string(), Value::String("C".into()))])?;
        Err::<(), _>(HelixiteError::Storage("simulated failure".into()))
    });

    assert!(result.is_err());
    assert_eq!(db.nodes().label("User").count().unwrap(), 0);
}

#[test]
fn test_batch_large_operation() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    db.batch(|tx| {
        for i in 0..100 {
            tx.add_node("Item", [("index".to_string(), Value::Int(i))])?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(db.nodes().label("Item").count().unwrap(), 100);
}

#[test]
fn test_batch_delete_then_read_in_same_txn() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let id = db.add_node("User", Vec::new()).unwrap();

    let result = db.batch(|tx| {
        tx.delete_node(id)?;
        tx.get_node(id)
    });

    assert!(matches!(result, Err(HelixiteError::NodeNotFound(_))));
}

#[test]
fn test_batch_add_edge_to_node_created_in_same_txn() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::new().open(dir.path()).unwrap();

    let (a, b, edge_id) = db
        .batch(|tx| {
            let a = tx.add_node("User", Vec::new())?;
            let b = tx.add_node("User", Vec::new())?;
            let edge = tx.add_edge(a, b, "knows", Vec::new())?;
            Ok((a, b, edge))
        })
        .unwrap();

    let edge = db.get_edge(edge_id).unwrap();
    assert_eq!(edge.from, a);
    assert_eq!(edge.to, b);
}

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
