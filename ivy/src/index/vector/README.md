# Vector index module

HNSW (Hierarchical Navigable Small World) vector index implementation.

## Submodules

```
keys.rs          — vector index key encoding
hnsw.rs          — HNSW algorithm implementation
similarity/      — similarity function dispatch
```

## Configuration

`HnswConfig` controls:
- `m` (default 16): links per node
- `ef_construction` (default 200): construction search width
- `ef_search` (default 50): query search width
- `similarity`: Cosine, DotProduct, Euclidean, or Custom
