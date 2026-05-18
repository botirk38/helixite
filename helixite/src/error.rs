use thiserror::Error;

#[derive(Error, Debug)]
pub enum HelixiteError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Codec error: {0}")]
    Codec(String),

    #[error("Node not found: {0}")]
    NodeNotFound(crate::id::NodeId),

    #[error("Edge not found: {0}")]
    EdgeNotFound(crate::id::EdgeId),

    #[error("Label not found: {0}")]
    LabelNotFound(String),

    #[error("Property not found: {0}")]
    PropertyNotFound(String),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Vector index not found: {label}::{property}")]
    VectorIndexNotFound { label: String, property: String },

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidVectorDim { expected: usize, actual: usize },

    #[error("Duplicate key: {0}")]
    DuplicateKey(String),

    #[error("Transaction conflict")]
    TransactionConflict,

    #[error("Invalid config: {0}")]
    InvalidConfig(String),
}

impl From<heed::Error> for HelixiteError {
    fn from(err: heed::Error) -> Self {
        HelixiteError::Storage(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, HelixiteError>;
