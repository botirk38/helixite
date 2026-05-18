use helixite::index::labels::LabelIndex;

#[test]
fn test_label_key_roundtrip() {
    let key = LabelIndex::key("User", 42);
    assert!(!key.is_empty());

    let decoded = LabelIndex::decode_node_id(&key).unwrap();
    assert_eq!(decoded, 42);
}

#[test]
fn test_label_prefix() {
    let prefix = LabelIndex::prefix("User");
    assert!(!prefix.is_empty());

    let key_a = LabelIndex::key("User", 1);
    let key_b = LabelIndex::key("User", 99);
    assert!(key_a.starts_with(&prefix));
    assert!(key_b.starts_with(&prefix));
}

#[test]
fn test_different_labels_dont_collide() {
    let prefix_a = LabelIndex::prefix("User");
    let prefix_b = LabelIndex::prefix("Post");
    assert_ne!(prefix_a, prefix_b);

    let key_a = LabelIndex::key("User", 1);
    let key_b = LabelIndex::key("Post", 1);
    assert!(!key_a.starts_with(&prefix_b));
    assert!(!key_b.starts_with(&prefix_a));
}

#[test]
fn test_decode_corrupt_key() {
    assert!(LabelIndex::decode_node_id(&[0, 0, 0]).is_none());
}

#[test]
fn test_label_key_empty_label() {
    let key = LabelIndex::key("", 1);
    assert!(!key.is_empty());
    let decoded = LabelIndex::decode_node_id(&key).unwrap();
    assert_eq!(decoded, 1);
}
