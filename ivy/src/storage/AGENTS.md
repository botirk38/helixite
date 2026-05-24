# Storage module (agent guidance)

## Purpose

Defines the `StorageEngine` trait and provides LMDB and in-memory backends.

## Safe to edit

- `engine.rs` — trait definitions, `Db` enum variants.
- `memory.rs` — in-memory backend (test-only).

## Edit with caution

- `lmdb.rs` — LMDB implementation. Changes may break durability or ordering.
- `env.rs` — LMDB environment setup.

## Invariants

- `LmdbStorage` opens all 8 `Db` databases on construction.
- `LmdbTxn` commits on success, aborts on drop.
- `MemoryStorage` uses `Arc<Mutex<HashMap>>` with snapshot isolation — no ordering guarantees.
- `MemoryStorage` tracks deleted keys in a `HashSet`, applies on commit.
- `ReadTxn::scan` has a default implementation that collects from `iter`.
- `WriteTxn` extends `ReadTxn` with `put` and `delete`.

## Adding a new backend

1. Implement `StorageEngine` trait.
2. Implement `ReadTxn` and `WriteTxn` for transaction types.
3. `ReadTxn::iter` must return entries in consistent order (required by index scans).
4. Add tests under `ivy/tests/storage/`.

## Dependencies

- `heed` crate for LMDB.
- `serde`/`bincode` for serialization.

## Tests

LMDB tests: `ivy/tests/storage/lmdb.rs`.
Memory tests: `ivy/tests/storage/memory.rs`.
