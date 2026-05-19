use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Db {
    Metadata,
    Nodes,
    Edges,
    OutEdges,
    InEdges,
    Labels,
    Properties,
    VectorIndexes,
}

impl Db {
    pub(crate) const ALL: [Db; 8] = [
        Db::Metadata,
        Db::Nodes,
        Db::Edges,
        Db::OutEdges,
        Db::InEdges,
        Db::Labels,
        Db::Properties,
        Db::VectorIndexes,
    ];

    pub(crate) const COUNT: usize = Self::ALL.len();

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Db::Metadata => "metadata",
            Db::Nodes => "nodes",
            Db::Edges => "edges",
            Db::OutEdges => "out_edges",
            Db::InEdges => "in_edges",
            Db::Labels => "labels",
            Db::Properties => "properties",
            Db::VectorIndexes => "vector_indexes",
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

/// Abstract read-only transaction over any logical database.
///
/// Implementations wrap concrete backend transactions (LMDB, in-memory, etc.)
/// and expose uniform read operations.
pub trait ReadTxn {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    fn iter_all(&self, db: Db) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

/// Abstract read-write transaction over any logical database.
///
/// Extends `ReadTxn` with mutation operations.
pub trait WriteTxn: ReadTxn {
    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()>;
}

/// Abstract storage engine for pluggable backends.
///
/// # Transaction model
///
/// All storage access happens inside a transaction.
///
/// `read` accepts a closure `FnOnce(&dyn ReadTxn) -> Result<T>`.
/// The engine opens a read transaction, passes it to the closure, and
/// provides a consistent snapshot for the duration of the closure.
///
/// `write` accepts a closure `FnOnce(&mut dyn WriteTxn) -> Result<T>`.
/// The engine opens a write transaction, passes it to the closure, and
/// commits only if the closure returns `Ok`. If the closure returns `Err`,
/// the transaction is aborted automatically.
pub trait StorageEngine: Send + Sync {
    fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn ReadTxn) -> Result<T>;
    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn WriteTxn) -> Result<T>;
}
