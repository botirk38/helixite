# Querying

## Nodes

```rust
// All nodes
let all: Vec<Node> = db.nodes().collect()?;

// By label
let users: Vec<Node> = db.nodes().label("User").collect()?;

// Property filter (requires index on (label, property))
let alice: Vec<Node> = db.nodes()
    .label("User")
    .eq("name", "Alice")
    .collect()?;

// Boolean operators
let adults: Vec<Node> = db.nodes()
    .label("User")
    .gte("age", 18)
    .lt("age", 65)
    .collect()?;

// Pagination
let page: Page<Node> = db.nodes()
    .label("User")
    .limit(20)
    .page(10)?;

// Cursor-based continuation
let next_page: Page<Node> = db.nodes()
    .label("User")
    .limit(20)
    .after(page.next_cursor.unwrap())
    .page(10)?;
```

## Edges

```rust
// All edges with label
let edges: Vec<Edge> = db.edges().label("knows").collect()?;

// Property filtered
let recent: Vec<Edge> = db.edges()
    .label("knows")
    .gte("since", 2024_i64.into())
    .collect()?;
```

## Traversal

```rust
// Single hop
let friends: Vec<Node> = db.node(alice)
    .outgoing("knows")
    .nodes()?;

// Single hop with property filter
let old_friends: Vec<Node> = db.node(alice)
    .outgoing("knows")
    .lt("since", 2023_i64.into())
    .nodes()?;

// Multi-hop
let friends_of_friends: Vec<NodeId> = db.node(alice)
    .outgoing("knows")
    .then_outgoing("knows")
    .ids()?;
```
