# Concepts

## Graph model

Ivy models data as a **labeled property graph**:

```
(User:Alice) —[knows]→ (User:Bob)
```

- **Nodes** have a single label and a map of string→Value properties.
- **Edges** are directed, with a source, target, label, and property map.
- **IDs** (`NodeId`, `EdgeId`) are `u64` values, assigned monotonically starting from 1.

## Values

Properties store `Value` variants:

| Variant | Rust type |
|---|---|
| `Value::String` | `String` |
| `Value::Int` | `i64` |
| `Value::Float` | `f64` |
| `Value::Bool` | `bool` |
| `Value::Bytes` | `Vec<u8>` |
| `Value::Vector` | `Vec<f32>` |

`Value::Vector` can only be stored on node properties and requires a vector index on `(label, property)`.

## Transactions

Ivy uses snapshot isolation: reads see a consistent snapshot of the database. Writes are buffered and atomically committed. See [Transactions](transactions.md).

## Indexes

- **Label index**: automatic — every node/edge is indexed by label for label-scoped queries.
- **Property index**: explicit — created via `db.indexes().nodes().create_property(label, property)`. Required for filter queries.
- **Unique property index**: enforces uniqueness of a property value per label.
- **Vector index**: explicit — created via `db.indexes().vectors().create(label, property, dim, config)`. Required for vector search.

## Storage backends

- **LMDB** (default): persistent, ACID-compliant, ordered key-value store.
- **MemoryStorage**: in-memory, for testing only.

Implement the `StorageEngine` trait for custom backends.
