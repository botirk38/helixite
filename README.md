# Ivy

An **embedded** graph-vector database for Rust.

Ivy runs in-process with no server, daemon, CLI, or HTTP API. It provides a labeled property graph with HNSW vector search, property indexes, and LMDB-backed persistence — all through a single Rust crate.

## What is Ivy?

Ivy is a self-contained Rust library for applications that need to store and query graph data with vector similarity search. It was inspired by HelixDB's graph + vector data model but is not wire/API compatible with HelixDB.

**Ivy does not include:**
- HTTP/gRPC server or API
- CLI tooling
- MCP or embedding integrations
- BM25 or full-text search
- Cloud or enterprise deployment

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
ivy = "0.1.0"
```

Or use the latest from Git:

```toml
[dependencies]
ivy = { git = "https://github.com/botirk38/ivy" }
```

```rust
use ivy::{IvyBuilder, Value, HnswConfig};

let db = IvyBuilder::new().open("path/to/db")?;

let alice = db.add_node("User", [("name", "Alice".into())])?;
let bob = db.add_node("User", [("name", "Bob".into())])?;
db.add_edge(alice, bob, "knows", [("since", 2024.into())])?;

let friends = db.node(alice).outgoing("knows").nodes()?;
```

## Features

- **Graph model**: nodes, edges, labels, properties, directional traversals
- **Vector search**: persisted HNSW indexes with cosine, dot product, and euclidean similarity
- **Property indexes**: indexed property lookups on nodes and edges with unique constraints
- **Persistence**: LMDB-backed durable storage; pluggable `StorageEngine` trait for custom backends
- **Transactions**: snapshot-isolated reads, atomic batch writes

## Documentation

- [Getting started](docs/getting-started.md)
- [Concepts](docs/concepts.md)
- [Storage](docs/storage.md)
- [Querying](docs/querying.md)
- [Indexing](docs/indexing.md)
- [Vector search](docs/vector-search.md)
- [Transactions](docs/transactions.md)
- [Examples](docs/examples.md)

## Crate

The library lives in `ivy/`. See the [crate README](ivy/README.md) for usage details.

## Development

From the `ivy/` directory:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo bench
```

## License

MIT
