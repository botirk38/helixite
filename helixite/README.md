# Helixite

An **embedded** graph-vector database for Rust, inspired by HelixDB's graph + vector data model.

Helixite runs **in-process** with no server, daemon, CLI, or HTTP API. It is not wire/API compatible with HelixDB.

## Features

- **Graph model**: nodes, edges, labels, properties, directional traversals.
- **Vector search**: persisted HNSW indexes with cosine, dot product, and euclidean similarity.
- **Property indexes**: indexed property lookups on nodes and edges.
- **Persistence**: LMDB-backed durable storage; pluggable `StorageEngine` trait for custom backends.
- **Transactions**: snapshot-isolated reads, atomic batch writes.

## Usage

```rust
use helixite::{HelixiteBuilder, Value, HnswConfig};

// Open or create a database
let mut db = HelixiteBuilder::new().open(path)?;

// Add nodes
let alice = db.add_node("User", [("name", Value::String("Alice".into()))])?;
let bob = db.add_node("User", [("name", Value::String("Bob".into()))])?;

// Add edges
db.add_edge(alice, bob, "knows", [("since", Value::Int(2024))])?;

// Traverse
let neighbors: Vec<_> = db.node(alice).outgoing("knows").nodes()?;

// Vector search
db.indexes().vectors().create("Chunk", "embedding", 384, HnswConfig::default())?;
let results = db.nodes().label("Chunk").nearest("embedding", query_vec, 10).ids()?;

// Batch write
db.batch(|tx| {
    tx.add_node("User", [("name", "Charlie".into())])
})?;
```

## Architecture

Helixite is a self-contained Rust library. It does not include:
- HTTP/gRPC server or API.
- CLI tooling.
- MCP or embedding integrations.
- BM25 or full-text search.
- Cloud or enterprise deployment.
- HelixDB dynamic query compatibility (`/v1/query`).

## License

MIT
