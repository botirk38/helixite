# Command module (agent guidance)

## Purpose

Orchestrates index creation, deletion, and backfill. Delegates to static methods on `VectorIndex`, `NodePropertyIndexes`, and `EdgePropertyIndexes`.

## Safe to edit

- `mod.rs` — re-exports only.

## Edit with caution

- `indexes.rs` — manager types. Adding a new index type requires corresponding implementation in `index/`.

## Dependencies

- `index/` — all index implementations.

## Adding a new index type

1. Implement in `index/`.
2. Add manager struct in `indexes.rs`.
3. Wire into `IndexManager`.
4. Add integration tests.
