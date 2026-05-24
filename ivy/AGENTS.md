# Crate: ivy

## Safe to edit

- `lib.rs` — re-exports only. Add new modules here.
- `config.rs` — `Config` struct.
- `node.rs` / `edge.rs` — plain data structs.
- `id.rs` — type aliases.
- `error.rs` — error enum variants.

## Edit with caution

- `db.rs` — top-level API, wraps all operations in transactions.
- `txn.rs` — transaction wrappers, mutation builders, ID allocation.
- `value.rs` — `Value` enum and `IndexedValue` binary encoding.
- `stats.rs` — full graph scan.

## Never edit

- LMDB internals in `storage/lmdb.rs` unless fixing storage bugs.
- HNSW internals in `index/vector/hnsw.rs` unless optimizing.

## Invariants

- `NodeId` / `EdgeId` are `u64`, 1-based; 0 is reserved/unused.
- Deleting a node cascades to all incident edges.
- `Value::Vector` cannot be property-indexed.
- Float encoding canonicalizes -0.0 to 0.0.
- Custom `SimilarityKind::Custom` cannot be persisted.
- `MemoryStorage` is for testing only.

## CI commands (run from `ivy/`)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo check --all-targets
```

## Dependencies

| Crate | Purpose |
|---|---|
| `heed` | LMDB bindings |
| `serde` | Serialization |
| `bincode` | Binary encoding |
| `thiserror` | Error derive |
| `rand` | HNSW random levels |

## Benchmarks

Run from `ivy/`:

```bash
cargo bench
```

Individual benchmarks: `cargo bench --bench node_writes`.
