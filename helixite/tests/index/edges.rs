use helixite::index::edges::EdgeIndex;

#[test]
fn test_out_key_roundtrip() {
    let key = EdgeIndex::out_key(1, "knows", 42);
    assert!(!key.is_empty());

    let decoded = EdgeIndex::decode_out_edge(&key).unwrap();
    assert_eq!(decoded.from, 1);
    assert_eq!(decoded.label, "knows");
    assert_eq!(decoded.edge_id, 42);
}

#[test]
fn test_in_key_roundtrip() {
    let key = EdgeIndex::in_key(2, "follows", 99);
    assert!(!key.is_empty());

    let decoded = EdgeIndex::decode_in_edge(&key).unwrap();
    assert_eq!(decoded.to, 2);
    assert_eq!(decoded.label, "follows");
    assert_eq!(decoded.edge_id, 99);
}

#[test]
fn test_out_prefix() {
    let prefix = EdgeIndex::out_prefix(1, Some("knows"));
    assert!(!prefix.is_empty());

    let full_key = EdgeIndex::out_key(1, "knows", 42);
    assert!(full_key.starts_with(&prefix));
}

#[test]
fn test_out_prefix_without_label() {
    let prefix = EdgeIndex::out_prefix(1, None);
    assert!(!prefix.is_empty());

    let key_a = EdgeIndex::out_key(1, "knows", 42);
    let key_b = EdgeIndex::out_key(1, "likes", 99);
    assert!(key_a.starts_with(&prefix));
    assert!(key_b.starts_with(&prefix));
}

#[test]
fn test_in_prefix() {
    let prefix = EdgeIndex::in_prefix(2, Some("follows"));
    assert!(!prefix.is_empty());

    let full_key = EdgeIndex::in_key(2, "follows", 99);
    assert!(full_key.starts_with(&prefix));
}

#[test]
fn test_out_key_different_labels_dont_collide() {
    let key_a = EdgeIndex::out_key(1, "knows", 42);
    let key_b = EdgeIndex::out_key(1, "likes", 42);
    assert_ne!(key_a, key_b);
}

#[test]
fn test_decode_corrupt_out_key() {
    assert!(EdgeIndex::decode_out_edge(&[0, 0, 0]).is_none());
}

#[test]
fn test_decode_corrupt_in_key() {
    assert!(EdgeIndex::decode_in_edge(&[0, 0, 0]).is_none());
}
