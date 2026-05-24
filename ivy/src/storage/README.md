# Storage module

Storage engine trait and backends (LMDB, in-memory).

## Submodules

```
engine.rs        — StorageEngine trait, Db enum, ReadTxn/WriteTxn traits
env.rs           — LMDB environment setup
lmdb.rs          — LmdbStorage implementation
memory.rs        — MemoryStorage (test-only)
```

## Db enum

| Variant | Purpose |
|---|---|
| Metadata | ID counters, index registry |
| Nodes | Node data (bincode serialized) |
| Edges | Edge data (bincode serialized) |
| OutEdges | Outgoing adjacency index |
| InEdges | Incoming adjacency index |
| Labels | Label index (nodes + edges) |
| Properties | Property index |
| VectorIndexes | HNSW vector index data |
