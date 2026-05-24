# Repo: helixite

Embedded graph-vector database for Rust. Single crate: `ivy`.

## Agent conventions

- All paths relative to repo root unless noted.
- CI commands run from `working-directory: ivy`:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --all-targets`
  - `cargo check --all-targets`
- Benchmarks live in `ivy/benches/` (criterion harness).
- Tests live in `ivy/tests/` (integration) and inline `#[cfg(test)]` modules.
- `cargo test --all-targets` runs 336+ tests.

## Invariants

- `NodeId` / `EdgeId` are `u64`, 1-based; 0 is reserved/unused.
- Deleting a node cascades to all incident edges (outbound + inbound).
- Vectors are not property-indexed; they go through the vector HNSW index.
- Property index queries require a label and an existing index on that property.
- Float encoding canonicalizes -0.0 to 0.0 for order-preserving binary sort.
- Custom `SimilarityKind::Custom` cannot be persisted (only Cosine/DotProduct/Euclidean survive restart).
- `MemoryStorage` is for testing only — no ordering guarantees.

## Public API types (in `ivy/src/lib.rs` re-exports)

`Ivy`, `IvyBuilder`, `IvyStorageBuilder`, `Config`, `Edge`, `Direction`, `IvyError`, `Result`, `NodeId`, `EdgeId`, `HnswConfig`, `SimilarityKind`, `Node`, `EdgeQuery`, `MultiHopTraversalQuery`, `NodeQuery`, `NodeRefQuery`, `Page`, `TraversalQuery`, `GraphStats`, `IndexStats`, `LabelStats`, `EdgeMut`, `EdgeMutBuilder`, `NodeMut`, `NodeMutBuilder`, `ReadTxn`, `WriteTxn`, `Value`.

## Module dependency graph

```
lib.rs
 ├── command/     → index/ (orchestrates index lifecycle)
 ├── config.rs
 ├── db.rs        → txn/, storage/, index/, command/
 ├── edge.rs
 ├── error.rs
 ├── id.rs
 ├── node.rs
 ├── stats.rs     → storage/ (full scan)
 ├── txn.rs       → storage/, index/
 ├── value.rs
 ├── index/
 │   ├── codec.rs
 │   ├── edges.rs
 │   ├── labels.rs
 │   ├── nodes.rs → labels/, properties/, vector/
 │   ├── properties.rs
 │   └── vector/
 │       ├── keys.rs
 │       ├── hnsw.rs
 │       └── similarity/
 │           ├── cosine.rs
 │           ├── dot.rs
 │           └── euclidean.rs
 ├── query/
 │   ├── edges.rs → storage/, index/properties
 │   ├── filter.rs
 │   ├── nodes.rs → storage/, index/labels, index/properties, index/vector
 │   ├── pagination.rs
 │   └── traversal/
 │       ├── single.rs → storage/, index/
 │       └── multi.rs
 └── storage/
     ├── engine.rs (traits)
     ├── env.rs
     ├── lmdb.rs
     └── memory.rs
```
