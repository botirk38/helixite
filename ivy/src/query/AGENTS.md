# Query module (agent guidance)

## Purpose

Fluent query builders that compile to storage scans and index lookups.

## Safe to edit

- `filter.rs` — `PropertyFilter` enum and `matches_indexed()` logic.
- `pagination.rs` — `Page` and `Cursor` encoding.

## Edit with caution

- `nodes.rs` — `NodeQuery` has three execution paths: label-only, property-filter, vector search.
- `edges.rs` — `EdgeQuery` mirrors `NodeQuery` without vector search.

## Invariants

- Property filter queries **require** a label and an existing property index — returns `IndexNotFound` otherwise.
- Vector search with filters: retrieves `k * 10` candidates from HNSW, then intersects with filter results.
- Cursor encoding: `"n:{id}"` for nodes, `"e:{id}"` for edges.
- Invalid cursors return `IvyError::InvalidCursor`.
- Cross-type filter comparisons safely return `false`.

## Dependencies

- `storage/` — scans `Db::Nodes`, `Db::Edges`, `Db::Properties`, `Db::Labels`.
- `index/` — property index registry, vector index search.
- `value.rs` — `IndexedValue` for filter matching.

## Adding a new filter operator

1. Add variant to `PropertyFilter` in `filter.rs`.
2. Add builder method to `NodeQuery` and `EdgeQuery`.
3. Implement `matches_indexed()` case.
4. Add tests.

## Tests

Node query tests: `ivy/tests/query/nodes.rs`.
Edge query tests: `ivy/tests/query/edges.rs`.
