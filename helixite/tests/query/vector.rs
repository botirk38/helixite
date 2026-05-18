use helixite::{HelixiteBuilder, Value};
use tempfile::tempdir;

#[test]
fn test_nearest_returns_ordered_results() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();
    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![0.0, 1.0, 0.0]))],
    )
    .unwrap();
    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![0.0, 0.0, 1.0]))],
    )
    .unwrap();

    let ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 3)
        .ids()
        .unwrap();

    assert_eq!(ids.len(), 3);
}

#[test]
fn test_nearest_with_k_limit() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    for i in 0..5 {
        db.add_node(
            "Chunk",
            vec![(
                "embedding".to_string(),
                Value::Vector(vec![i as f32, 0.0, 0.0]),
            )],
        )
        .unwrap();
    }

    let ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![2.0, 0.0, 0.0], 2)
        .ids()
        .unwrap();

    assert_eq!(ids.len(), 2);
}

#[test]
fn test_nearest_empty_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    let ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();

    assert_eq!(ids.len(), 0);
}

#[test]
fn test_nearest_with_label_filter() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();
    db.add_node(
        "Doc",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();

    let chunk_ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();

    let doc_ids = db
        .nodes()
        .label("Doc")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();

    assert_eq!(chunk_ids.len(), 1);
    assert_eq!(doc_ids.len(), 1);
    assert_ne!(chunk_ids[0], doc_ids[0]);
}

#[test]
fn test_nearest_with_property_filter() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node(
        "Chunk",
        vec![
            ("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0])),
            ("status".to_string(), Value::String("active".to_string())),
        ],
    )
    .unwrap();
    db.add_node(
        "Chunk",
        vec![
            ("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0])),
            ("status".to_string(), Value::String("archived".to_string())),
        ],
    )
    .unwrap();

    let active_ids = db
        .nodes()
        .label("Chunk")
        .where_eq("status", Value::String("active".to_string()))
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();

    assert_eq!(active_ids.len(), 1);
}

#[test]
fn test_nearest_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.add_node(
            "Chunk",
            vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
        )
        .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();

    assert_eq!(ids.len(), 1);
}

#[test]
fn test_nearest_requires_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();

    let result = db
        .nodes()
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids();
    assert!(result.is_err());
}
