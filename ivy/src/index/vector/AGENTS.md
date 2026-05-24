# Vector index module (agent guidance)

## Purpose

Approximate nearest-neighbor search via HNSW. Persists vector data, levels, and neighbor links in `Db::VectorIndexes`.

## Key encoding

Vector index keys use prefix bytes in `Db::VectorIndexes`:
- `0` = metadata, `1` = vector data, `2` = node level, `3` = link/neighbor, `4` = entry point.

## Safe to edit

- `keys.rs` — key encoding utilities.
- `mod.rs` — `HnswConfig` builder and `VectorIndex` orchestration.

## Edit with caution

- `hnsw.rs` — the core HNSW algorithm. Changes affect search quality and performance.

## Invariants

- `VectorIndex::create` rejects `SimilarityKind::Custom` — cannot be persisted.
- Random level uses geometric distribution based on `m`, deterministic per `node_id`.
- Entry point election: when deleted, highest-level remaining node becomes new entry point.
- Links are bidirectional — inserting a link from A to B also adds B to A's neighbors.
- Pruning: when degree exceeds `m`, keeps top-m neighbors by similarity.

## Dependencies

- `similarity/` — distance function dispatch.
- `storage/` — `Db::VectorIndexes` database.

## Tests

Vector search tests: `ivy/tests/query/vector.rs`.
Benchmarks: `ivy/benches/vector.rs`.
