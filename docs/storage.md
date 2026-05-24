# Storage

## LMDB (default)

Ivy uses LMDB via the `heed` crate. Configuration is set through `Config`:

```rust
use ivy::{IvyBuilder, Config};

let config = Config::default()
    .with_map_size(2 * 1024 * 1024 * 1024); // 2 GiB

let db = IvyBuilder::new()
    .config(config)
    .open("path/to/db")?;
```

### Config fields

| Field | Default | Description |
|---|---|---|
| `map_size` | 1 GiB | Maximum memory map size |
| `max_dbs` | 32 | Maximum named databases |
| `max_readers` | 126 | Maximum concurrent readers |

### Internal databases

Ivy manages 8 LMDB databases internally: `Metadata`, `Nodes`, `Edges`, `OutEdges`, `InEdges`, `Labels`, `Properties`, `VectorIndexes`.

## MemoryStorage

For testing or ephemeral use:

```rust
use ivy::{IvyBuilder, storage::MemoryStorage};

let db = IvyBuilder::new()
    .storage(MemoryStorage::default())
    .open("")?;
```

`MemoryStorage` uses snapshot isolation on an `Arc<Mutex<HashMap>>`. No ordering guarantees.

## Custom engine

Implement the `StorageEngine` trait in `ivy::storage::engine`:

```rust
use ivy::storage::{StorageEngine, ReadTxn, WriteTxn, Db};
```
