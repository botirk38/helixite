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

pub struct Entry<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
}

pub type EntryIter<'a> = Box<dyn Iterator<Item = Result<Entry<'a>>> + 'a>;

pub enum Scan<'a> {
    All,
    Prefix(&'a [u8]),
}

pub trait ReadTxn {
    fn get(&self, db: Db, key: &[u8]) -> Result<Option<Vec<u8>>>;

    fn iter<'a>(&'a self, db: Db, scan: Scan<'a>) -> Result<EntryIter<'a>>;

    fn scan<'a>(&'a self, db: Db, scan: Scan<'a>, limit: Option<usize>) -> Result<Vec<Entry<'a>>> {
        match limit {
            Some(limit) => self.iter(db, scan)?.take(limit).collect(),
            None => self.iter(db, scan)?.collect(),
        }
    }
}

pub trait WriteTxn: ReadTxn {
    fn put(&mut self, db: Db, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, db: Db, key: &[u8]) -> Result<()>;
}

pub trait StorageEngine: Send + Sync {
    fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn ReadTxn) -> Result<T>;
    fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn WriteTxn) -> Result<T>;

    fn close(&self) -> Result<()> {
        Ok(())
    }
}
