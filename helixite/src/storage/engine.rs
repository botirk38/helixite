use crate::error::Result;

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

pub trait StorageTxn {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()>;
    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

pub trait StorageEngine: Send + Sync {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn scan_prefix(&self, db: Db, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn StorageTxn) -> Result<T>;
}
