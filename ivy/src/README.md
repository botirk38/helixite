# Source modules

All source code lives under `ivy/src/`.

## Module structure

```
lib.rs             — crate root, re-exports
config.rs          — Config struct
db.rs              — Ivy, IvyBuilder, IvyStorageBuilder
edge.rs            — Edge, Direction
error.rs           — IvyError, Result
id.rs              — NodeId, EdgeId
node.rs            — Node struct
stats.rs           — GraphStats, LabelStats, IndexStats
txn.rs             — ReadTxn, WriteTxn, NodeMut, EdgeMut
value.rs           — Value enum, IndexedValue encoding
command/           — index management orchestration
index/             — index implementations
query/             — query builders and executors
storage/           — storage engine trait + backends
```

See `AGENTS.md` for edit boundaries, invariants, and CI commands.
