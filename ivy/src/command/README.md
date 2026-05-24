# Command module

Orchestrates index lifecycle management. Provides `IndexManager` with sub-managers for vector, node property, and edge property indexes.

## Public API

- `IndexManager<S>` — top-level manager, accessed via `db.indexes()`.
  - `.vectors()` → `VectorIndexManager`
  - `.nodes()` → `NodeIndexManager`
  - `.edges()` → `EdgeIndexManager`
