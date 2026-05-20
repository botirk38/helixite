use crate::error::{HelixiteError, Result};
use crate::id::{EdgeId, NodeId};

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

impl<T> Page<T> {
    /// Build a page from an ordered iterator.
    ///
    /// `after` is the cursor to skip past. `matches` checks if an item
    /// corresponds to the cursor. `cursor` produces a cursor from an item.
    ///
    /// Returns `InvalidCursor` if `after` is provided but no item matches.
    pub(crate) fn from_iter<I, M, C>(
        iter: I,
        limit: usize,
        after: Option<&Cursor>,
        matches: M,
        cursor: C,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        M: Fn(&T) -> bool,
        C: Fn(&T) -> String,
    {
        let mut it = iter.into_iter();
        let mut items = Vec::with_capacity(limit);

        if after.is_some() {
            let mut cursor_found = false;
            for item in it.by_ref() {
                if matches(&item) {
                    cursor_found = true;
                    break;
                }
            }
            if !cursor_found {
                return Err(HelixiteError::InvalidCursor(
                    "cursor not found in result set".into(),
                ));
            }
        }

        let mut has_more = false;
        for item in it {
            if items.len() == limit {
                has_more = true;
                break;
            }
            items.push(item);
        }

        let next_cursor = if has_more {
            items.last().map(cursor)
        } else {
            None
        };

        Ok(Self { items, next_cursor })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cursor {
    Node(NodeId),
    Edge(EdgeId),
}

impl Cursor {
    pub(crate) fn encode_node(id: NodeId) -> String {
        format!("n:{id}")
    }

    pub(crate) fn encode_edge(id: EdgeId) -> String {
        format!("e:{id}")
    }

    pub(crate) fn decode_node(s: &str) -> Result<Self> {
        let rest = s.strip_prefix("n:").ok_or_else(|| {
            HelixiteError::InvalidCursor("node cursor must start with 'n:'".into())
        })?;
        let node_id = rest.parse().map_err(|_| {
            HelixiteError::InvalidCursor(format!("invalid node id in cursor: {rest}"))
        })?;
        Ok(Cursor::Node(node_id))
    }

    pub(crate) fn decode_edge(s: &str) -> Result<Self> {
        let rest = s.strip_prefix("e:").ok_or_else(|| {
            HelixiteError::InvalidCursor("edge cursor must start with 'e:'".into())
        })?;
        let edge_id = rest.parse().map_err(|_| {
            HelixiteError::InvalidCursor(format!("invalid edge id in cursor: {rest}"))
        })?;
        Ok(Cursor::Edge(edge_id))
    }

    pub(crate) fn matches_node(&self, id: NodeId) -> bool {
        matches!(self, Cursor::Node(nid) if *nid == id)
    }

    pub(crate) fn matches_edge(&self, id: EdgeId) -> bool {
        matches!(self, Cursor::Edge(eid) if *eid == id)
    }
}
