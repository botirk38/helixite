# Vector search

Ivy uses HNSW (Hierarchical Navigable Small World) for approximate nearest-neighbor search.

## Creating a vector index

```rust
use ivy::HnswConfig;

db.indexes().vectors().create(
    "Chunk",        // node label
    "embedding",    // property name
    384,            // vector dimension
    HnswConfig::default(),
)?;
```

## HNSW configuration

| Field | Default | Description |
|---|---|---|
| `m` | 16 | Number of bi-directional links per node |
| `ef_construction` | 200 | Search width during index construction |
| `ef_search` | 50 | Search width during query |
| `similarity` | Cosine | Distance function |

```rust
// Cosine similarity (default)
let config = HnswConfig::cosine();

// Dot product
let config = HnswConfig::dot_product();

// Euclidean distance
let config = HnswConfig::euclidean();

// Custom (not persistable)
let config = HnswConfig::custom(|a, b| Ok(dot(a, b)));
```

## Querying

```rust
let query_vec: Vec<f32> = vec![0.1, 0.2, /* ... */];

let results: Vec<NodeId> = db.nodes()
    .label("Chunk")
    .nearest("embedding", query_vec, 10) // top 10
    .ids()?;
```

Vector search can be combined with property filters:

```rust
let results: Vec<NodeId> = db.nodes()
    .label("Chunk")
    .eq("source", "document_42")
    .nearest("embedding", query_vec, 10)
    .ids()?;
```

When filters are present, Ivy retrieves `k * 10` candidates from HNSW and intersects with filtered results.

## Similarity metrics

| Kind | Range | Higher = better |
|---|---|---|
| Cosine | [-1, 1] | Yes |
| Dot product | (-∞, ∞) | Yes |
| Euclidean | [0, ∞) | No (lower = closer) |
