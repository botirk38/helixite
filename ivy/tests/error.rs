use ivy::error::IvyError;

#[test]
fn test_error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = IvyError::Io(io_err);
    assert_eq!(format!("{err}"), "IO error: file not found");
}

#[test]
fn test_error_display_storage() {
    let err = IvyError::Storage("lmdb map full".into());
    assert_eq!(format!("{err}"), "Storage error: lmdb map full");
}

#[test]
fn test_error_display_codec() {
    let err = IvyError::Codec("bincode deserialize failed".into());
    assert_eq!(format!("{err}"), "Codec error: bincode deserialize failed");
}

#[test]
fn test_error_display_node_not_found() {
    let err = IvyError::NodeNotFound(42);
    assert_eq!(format!("{err}"), "Node not found: 42");
}

#[test]
fn test_error_display_edge_not_found() {
    let err = IvyError::EdgeNotFound(99);
    assert_eq!(format!("{err}"), "Edge not found: 99");
}

#[test]
fn test_error_display_label_not_found() {
    let err = IvyError::LabelNotFound("User".into());
    assert_eq!(format!("{err}"), "Label not found: User");
}

#[test]
fn test_error_display_property_not_found() {
    let err = IvyError::PropertyNotFound("email".into());
    assert_eq!(format!("{err}"), "Property not found: email");
}

#[test]
fn test_error_display_index_not_found() {
    let err = IvyError::IndexNotFound("idx_email".into());
    assert_eq!(format!("{err}"), "Index not found: idx_email");
}

#[test]
fn test_error_display_vector_index_not_found() {
    let err = IvyError::VectorIndexNotFound {
        label: "Chunk".into(),
        property: "embedding".into(),
    };
    assert_eq!(format!("{err}"), "Vector index not found: Chunk::embedding");
}

#[test]
fn test_error_display_invalid_vector_dim() {
    let err = IvyError::InvalidVectorDim {
        expected: 1536,
        actual: 768,
    };
    assert_eq!(
        format!("{err}"),
        "Invalid vector dimension: expected 1536, got 768"
    );
}

#[test]
fn test_error_display_duplicate_key() {
    let err = IvyError::DuplicateKey("unique_email".into());
    assert_eq!(format!("{err}"), "Duplicate key: unique_email");
}

#[test]
fn test_error_display_transaction_conflict() {
    let err = IvyError::TransactionConflict;
    assert_eq!(format!("{err}"), "Transaction conflict");
}

#[test]
fn test_error_display_invalid_config() {
    let err = IvyError::InvalidConfig("map_size too small".into());
    assert_eq!(format!("{err}"), "Invalid config: map_size too small");
}

#[test]
fn test_error_display_invalid_cursor() {
    let err = IvyError::InvalidCursor("cursor not found in result set".into());
    assert_eq!(
        format!("{err}"),
        "Invalid pagination cursor: cursor not found in result set"
    );
}

#[test]
fn test_error_debug() {
    let err = IvyError::Storage("test".into());
    let debug = format!("{err:?}");
    assert!(debug.contains("Storage"));
}

#[test]
fn test_from_io_error() {
    let io_err = std::io::Error::other("io");
    let err: IvyError = io_err.into();
    assert!(matches!(err, IvyError::Io(_)));
}
