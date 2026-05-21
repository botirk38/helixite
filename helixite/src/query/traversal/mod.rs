mod multi;
mod single;

pub use multi::MultiHopTraversalQuery;
pub(crate) use single::EdgePropertyFilter;
pub use single::{NodeRefQuery, TraversalQuery};

use crate::edge::Direction;
use crate::id::NodeId;
use crate::index::edges::EdgeIndex;
use crate::storage::engine::Db;

#[derive(Clone)]
struct TraversalHop {
    direction: Direction,
    label: Option<String>,
}

impl TraversalHop {
    fn prefix_and_db(&self, node_id: NodeId) -> (Db, Vec<u8>) {
        match self.direction {
            Direction::Out => (
                Db::OutEdges,
                EdgeIndex::out_prefix(node_id, self.label.as_deref()),
            ),
            Direction::In => (
                Db::InEdges,
                EdgeIndex::in_prefix(node_id, self.label.as_deref()),
            ),
        }
    }
}
