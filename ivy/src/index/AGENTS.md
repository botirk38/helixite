# Index module (agent guidance)

## Purpose

Manages all index types: label, property, vector. Each index writes to specific `Db` databases via the storage engine.

## Key encoding

All indexes use `KeyBuilder`/`KeyReader` from `codec.rs` for lexicographically sortable composite keys.

## Safe to edit

- `labels.rs` — label index key encoding.
- `codec.rs` — key builder utilities.

## Edit with caution

- `properties.rs` — property indexes with unique constraints, backfill logic.
- `nodes.rs` — orchestrates label + property + vector index updates on node changes.
- `edges.rs` — adjacency index key encoding.

## Invariants

- Label index keys for nodes and edges share `Db::Labels` — edge keys are prefixed with `0x01` to avoid collision.
- Property index metadata uses tag bytes: `0` (node prop), `1` (edge prop), `2` (unique node), `3` (unique edge).
- Backfill scans all existing entities of a label — can be expensive.
- Unique validation must run before marking an index as unique.

## Dependencies

- `storage/` — reads/writes to `Db` databases.
- `value.rs` — `IndexedValue` for property index encoding.

## Tests

Property index tests: `ivy/tests/command/indexes.rs`.
Vector index tests: `ivy/tests/query/vector.rs`.
