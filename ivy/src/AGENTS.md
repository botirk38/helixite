# Source modules (agent guidance)

## Module visibility

Public modules (re-exported in `lib.rs`): `config`, `db`, `edge`, `error`, `id`, `node`, `stats`, `storage`, `value`.

Private modules: `command`, `index`, `query`, `txn`.

## Cross-module dependencies

```
db.rs → txn/, storage/, index/, command/
txn.rs → storage/, index/
stats.rs → storage/
command/ → index/
query/ → storage/, index/
index/ → storage/ (writes to Db databases)
```

## Adding a new module

1. Declare in `lib.rs` (pub or private).
2. Add re-exports to `lib.rs` if public.
3. Add tests under `ivy/tests/`.
4. Run `cargo test --all-targets`.

## Safe to edit

- Data structs: `node.rs`, `edge.rs`, `id.rs`, `config.rs`.
- Error variants: `error.rs`.

## Edit with caution

- `db.rs` — wraps all operations in transactions.
- `txn.rs` — transaction semantics, ID allocation, node delete cascade.
- `value.rs` — binary encoding affects index ordering.

## Tests

Integration tests: `ivy/tests/`.
Inline tests: `#[cfg(test)]` modules in source files.
