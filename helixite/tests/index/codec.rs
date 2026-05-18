use helixite::id::NodeId;

fn label_key(label: &str, node_id: NodeId) -> Vec<u8> {
    let mut key = Vec::new();
    key.push(0);
    key.extend((label.len() as u32).to_be_bytes());
    key.extend(label.as_bytes());
    key.extend(node_id.to_be_bytes());
    key
}

#[test]
fn test_key_builder_str() {
    let key = label_key("User", 42);
    assert!(!key.is_empty());
}

#[test]
fn test_key_builder_u64() {
    let key = label_key("User", u64::MAX);
    assert!(!key.is_empty());
}

#[test]
fn test_label_prefix_does_not_collide() {
    let user_key = label_key("User", 1);
    let user_profile_key = label_key("UserProfile", 1);

    assert_ne!(user_key[..5], user_profile_key[..5]);
}
