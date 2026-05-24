# Transactions

## Read transactions

Every read operation on `Ivy` opens a temporary read transaction internally. For multiple reads under the same snapshot:

```rust
let tx = db.read()?;
let node1 = tx.get_node(id1)?;
let node2 = tx.get_node(id2)?;
// tx drops here, releasing the snapshot
```

## Write transactions

Single writes (`add_node`, `add_edge`, `delete_node`, etc.) each open and commit a write transaction internally.

For atomic batch writes:

```rust
db.batch(|tx| {
    let alice = tx.add_node("User", [("name", "Alice".into())])?;
    let bob = tx.add_node("User", [("name", "Bob".into())])?;
    tx.add_edge(alice, bob, "knows", [("since", 2024.into())])?;
    Ok(())
})?;
```

If the closure returns an error, the transaction is aborted and all changes are discarded.

## Mutation builders

For updating existing entities:

```rust
db.update_node(node_id)
    .set_property("age", 31.into())
    .remove_property("old_field")
    .apply()?;

db.update_edge(edge_id)
    .set_property("weight", 0.5.into())
    .apply()?;
```

## Isolation

Ivy provides snapshot isolation for LMDB reads. Write transactions are serialized.
