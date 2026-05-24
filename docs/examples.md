# Examples

## Basic CRUD

```rust
use ivy::{IvyBuilder, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = IvyBuilder::new().open("/tmp/my-db")?;

    // Create
    let alice = db.add_node("User", [
        ("name", "Alice".into()),
        ("age", 30.into()),
    ])?;
    let bob = db.add_node("User", [
        ("name", "Bob".into()),
        ("age", 25.into()),
    ])?;
    db.add_edge(alice, bob, "knows", [("since", 2024.into())])?;

    // Read
    let node = db.get_node(alice)?.unwrap();
    assert_eq!(node.label, "User");

    // Update
    db.update_node(alice).set_property("age", 31.into()).apply()?;

    // Delete
    db.delete_node(bob)?; // cascades to edges

    Ok(())
}
```

## Property filter

```rust
db.indexes().nodes().create_property("User", "age")?;

let adults = db.nodes()
    .label("User")
    .gte("age", 18_i64.into())
    .collect()?;
```

## Vector search

```rust
db.indexes().vectors().create(
    "Chunk", "embedding", 128, HnswConfig::default()
)?;

let query: Vec<f32> = vec![0.1; 128];
let similar = db.nodes()
    .label("Chunk")
    .nearest("embedding", query, 5)
    .ids()?;
```

## Multi-hop traversal

```rust
let coauthors: Vec<NodeId> = db.node(author)
    .outgoing("wrote")
    .then_incoming("wrote")
    .ids()?;
```

## Stats

```rust
let stats = db.stats()?;
println!("{} nodes, {} edges", stats.node_count, stats.edge_count);
for ls in &stats.labels {
    println!("  {}: {} nodes", ls.label, ls.node_count);
}
```
