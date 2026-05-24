# Query module

Query builders and executors for nodes, edges, and graph traversals.

## Submodules

```
edges.rs         — EdgeQuery builder
filter.rs        — PropertyFilter types
nodes.rs         — NodeQuery builder (includes vector search)
pagination.rs    — Page, Cursor types
traversal/       — single-hop and multi-hop traversals
```

## Query execution

All query builders are lazy — they only execute when a terminal method is called (`.collect()`, `.ids()`, `.count()`, `.first()`, `.page()`).
