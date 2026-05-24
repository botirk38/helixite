# Getting started

Add Ivy to your `Cargo.toml`:

```toml
[dependencies]
ivy = { git = "https://github.com/botirk38/helixite" }
```

Open or create a database:

```rust
use ivy::{IvyBuilder, Value};

let db = IvyBuilder::new().open("path/to/db")?;
```

`IvyBuilder::new()` defaults to LMDB persistent storage. Use `IvyBuilder::new().storage(custom_engine)` to plug in a custom backend.

## Key concepts

- **Nodes**: entities with a label and property map.
- **Edges**: directed relationships between two nodes, with a label and property map.
- **Indexes**: property indexes for fast lookups, vector indexes for ANN search.
- **Transactions**: snapshot-isolated reads, atomic batch writes.

See [Concepts](concepts.md) for a full explanation.
