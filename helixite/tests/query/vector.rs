use helixite::{HelixiteBuilder, HnswConfig, Value};
use tempfile::tempdir;

#[test]
fn test_create_vector_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();
}

#[test]
fn test_vector_index_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
            .unwrap();
    }

    let db = HelixiteBuilder::default().open(path).unwrap();
    let result = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_add_node_with_vector() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

    let node_id = db
        .add_node(
            "Chunk",
            vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
        )
        .unwrap();

    let node = db.get_node(node_id).unwrap();
    assert!(matches!(node.properties.get("embedding"), Some(Value::Vector(v)) if v.len() == 3));
}

#[test]
fn test_nearest_returns_ordered_results() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

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
    assert_eq!(ids[0], 1);
}

#[test]
fn test_nearest_with_k_limit() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

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

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

    let ids = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids()
        .unwrap();

    assert_eq!(ids.len(), 0);
}

#[test]
fn test_nearest_requires_label() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

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

#[test]
fn test_nearest_requires_existing_index() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();

    let result = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 5)
        .ids();
    assert!(result.is_err());
}

#[test]
fn test_dimension_mismatch_on_insert() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

    let result = db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0]))],
    );
    assert!(result.is_err());
}

#[test]
fn test_dimension_mismatch_on_search() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();

    let result = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0], 5)
        .ids();
    assert!(result.is_err());
}

#[test]
fn test_nearest_with_property_filter() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

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
            ("embedding".to_string(), Value::Vector(vec![0.9, 0.0, 0.0])),
            ("status".to_string(), Value::String("archived".to_string())),
        ],
    )
    .unwrap();
    db.add_node(
        "Chunk",
        vec![
            ("embedding".to_string(), Value::Vector(vec![0.8, 0.0, 0.0])),
            ("status".to_string(), Value::String("active".to_string())),
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

    assert_eq!(active_ids.len(), 2);
}

#[test]
fn test_nearest_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let db = HelixiteBuilder::default().open(path).unwrap();
        db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
            .unwrap();
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
fn test_multiple_vector_indexes() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();
    db.create_vector_index("Doc", "vector", 2, HnswConfig::default())
        .unwrap();

    db.add_node(
        "Chunk",
        vec![("embedding".to_string(), Value::Vector(vec![1.0, 0.0, 0.0]))],
    )
    .unwrap();
    db.add_node(
        "Doc",
        vec![("vector".to_string(), Value::Vector(vec![0.0, 1.0]))],
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
        .nearest("vector", vec![0.0, 1.0], 5)
        .ids()
        .unwrap();

    assert_eq!(chunk_ids.len(), 1);
    assert_eq!(doc_ids.len(), 1);
}

#[test]
fn test_nearest_collect_preserves_order() {
    let dir = tempdir().unwrap();
    let db = HelixiteBuilder::default().open(dir.path()).unwrap();

    db.create_vector_index("Chunk", "embedding", 3, HnswConfig::default())
        .unwrap();

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

    let nodes = db
        .nodes()
        .label("Chunk")
        .nearest("embedding", vec![1.0, 0.0, 0.0], 3)
        .collect()
        .unwrap();

    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].id, 1);
}
