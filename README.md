# NyxDB

NyxDB is a lightweight, embedded key-value storage engine written in Rust. It is designed to be a persistent, crash-safe database built from the ground up, implementing a Log-Structured Merge-tree (LSM-tree) architecture.

> [!NOTE]
> This project is currently in the **starting stage** of development. Core components are implemented, but features like compaction, bloom filters, and advanced caching are planned for future updates.

## Features

-   **LSM-Tree Architecture**: Optimized for write-heavy workloads.
-   **Persistence**: Uses a Write-Ahead Log (WAL) to ensure data durability.
-   **Crash Recovery**: Automatically recovers state from the WAL upon restart.
-   **In-Memory Buffer**: Uses a MemTable for fast writes and recent reads.
-   **SSTables**: Flushes data to Sorted String Tables (SSTables) on disk when the MemTable fills up.

## components

The project is structured into the following core modules:

-   **MemTable** (`src/memtable`): In-memory storage using a balanced tree (BTreeMap).
-   **WAL** (`src/wal`): Write-Ahead Log for recording operations before they are committed.
-   **SSTable** (`src/sstable`): Immutable on-disk file format for long-term storage.
-   **Recov** (`src/recov`): Logic for recovering database state from the WAL.
-   **DB** (`src/db`): The main public API for interacting with the database.

## Usage

Here is a simple example of how to use NyxDB:

```rust
use nyxdb::db::NyxDB;

fn main() -> std::io::Result<()> {
    // Open a database instance at the specified path
    let mut db = NyxDB::open("./my_db")?;

    // specific Key and Value pairs
    let key = b"username".to_vec();
    let value = b"admin".to_vec();

    // Store a key-value pair
    db.put(key.clone(), value)?;

    // Retrieve the value
    if let Some(retrieved_value) = db.get(&key) {
        println!("Found value: {:?}", String::from_utf8_lossy(&retrieved_value));
    } else {
        println!("Key not found");
    }

    // Delete the key
    db.delete(key)?;

    Ok(())
}
```

## Roadmap

- [ ] Bloom Filters for faster lookups.
- [ ] Leveled Compaction to reclaim space and optimize reads.
- [ ] Block Cache for improved read performance.
- [ ] Iterator support for range queries.

## Contributing

Contributions are welcome! Since this is an early-stage project, please open an issue to discuss major changes before implementing them.

## License

[MIT](LICENSE)
