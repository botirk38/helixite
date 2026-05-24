# Traversal module

Single-hop and multi-hop graph traversals starting from a node reference.

## Submodules

```
single.rs        — NodeRefQuery, TraversalQuery (single hop)
multi.rs         — MultiHopTraversalQuery (n-step)
```

## Usage

```rust
// Single hop
db.node(id).outgoing("knows").nodes()?;

// Multi-hop
db.node(id).outgoing("knows").then_outgoing("knows").ids()?;
```
