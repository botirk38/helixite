/// Unique identifier for a node in the graph.
///
/// IDs start at 1 and increment monotonically. The value 0 is reserved
/// and never assigned — do not use `NodeId::default()` as a valid key.
pub type NodeId = u64;

/// Unique identifier for an edge in the graph.
///
/// IDs start at 1 and increment monotonically. The value 0 is reserved
/// and never assigned — do not use `EdgeId::default()` as a valid key.
pub type EdgeId = u64;
