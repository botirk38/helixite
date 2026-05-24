# Traversal module (agent guidance)

## Purpose

Graph traversal starting from `db.node(id)`, which returns `NodeRefQuery`. Single-hop traversals support property filters; multi-hop traversals do not.

## Safe to edit

- `mod.rs` — re-exports only.

## Edit with caution

- `single.rs` — `NodeRefQuery`, `TraversalQuery`, `TraversalExec`. Property filters on traversals require index intersection.
- `multi.rs` — `MultiHopTraversalQuery`, `MultiHopTraversalExec`. Uses `BTreeSet` for deduplication at each hop.

## Invariants

- Single-hop traversals resolve edges by prefix-scanning `Db::OutEdges` or `Db::InEdges`.
- When property filters are present on single-hop: first resolves candidate edge IDs from property indexes, then intersects with adjacency IDs.
- Multi-hop traversals have **no property filter support** on intermediate hops — this is a design limitation.
- Multi-hop uses `BTreeSet` at each step to deduplicate node IDs.

## Dependencies

- `storage/` — adjacency index scans.
- `index/` — property index intersection.

## Tests

Traversal tests: `ivy/tests/query/traversal.rs`.
Benchmarks: `ivy/benches/traversal.rs`.
