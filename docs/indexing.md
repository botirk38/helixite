# Indexing

## Property indexes

Property indexes enable filter queries (`eq`, `ne`, `gt`, `gte`, `lt`, `lte`, `in_`).

```rust
// Create a property index on (User, name)
db.indexes().nodes().create_property("User", "name")?;

// Create a unique property index
db.indexes().nodes().create_unique("User", "email")?;

// Drop an index
db.indexes().nodes().drop_property("User", "name")?;
```

Indexes are **required** for property filters — querying without an index returns `IndexNotFound`.

### Edge property indexes

```rust
db.indexes().edges().create_property("knows", "since")?;
```

## Index registry

Index metadata is persisted. Query `db.stats()?` to inspect which indexes exist.

## Backfill

Creating an index on an existing label backfills all existing entities. This can be expensive for large datasets.
