use helixite::{EdgeId, NodeId};

#[test]
fn test_node_id_is_u64() {
    let id: NodeId = 42;
    assert_eq!(id, 42u64);
}

#[test]
fn test_edge_id_is_u64() {
    let id: EdgeId = 99;
    assert_eq!(id, 99u64);
}

#[test]
fn test_node_id_zero() {
    let id: NodeId = 0;
    assert_eq!(id, 0);
}

#[test]
fn test_edge_id_max() {
    let id: EdgeId = u64::MAX;
    assert_eq!(id, u64::MAX);
}

#[test]
fn test_node_id_arithmetic() {
    let id: NodeId = 1;
    assert_eq!(id + 1, 2);
}
