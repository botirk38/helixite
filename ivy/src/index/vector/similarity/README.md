# Similarity module

Similarity function dispatch and implementations for vector search.

## Files

```
mod.rs           — SimilarityKind enum, dispatch
cosine.rs        — cosine similarity
dot.rs           — dot product
euclidean.rs     — euclidean distance
```

## SimilarityKind

| Variant | Persistable | Higher = better |
|---|---|---|
| Cosine | Yes | Yes |
| DotProduct | Yes | Yes |
| Euclidean | Yes | No |
| Custom(fn) | No | Depends |
