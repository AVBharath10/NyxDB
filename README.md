# NyxDB

NyxDB is a lightweight, embedded key-value storage engine written in Rust. It is designed to be a persistent, crash-safe database built from the ground up, implementing a Log-Structured Merge-tree (LSM-tree) architecture.
(Because the world clearly needed **onew more** LSM-tree database.

> [!NOTE]
> This project is currently in the **starting stage** of development. Core components are implemented, but features like compaction, bloom filters, and advanced caching are planned for future updates.

## Features

- **LSM-tree based design**  
  Writes go to memory first and hit disk sequentially, making NyxDB naturally suited for write-heavy workloads.

- **Durable by default (WAL)**  
  Every operation is written to a Write-Ahead Log before it’s applied, so data isn’t lost even if the process crashes midway.

- **Crash recovery that actually works**  
  On restart, NyxDB replays the WAL and rebuilds its state automatically — no manual intervention needed.

- **In-memory buffering with MemTable**  
  Recent writes live in memory using a sorted structure, keeping writes fast and reads efficient.

- **Immutable SSTables on disk**  
  When the MemTable fills up, its contents are flushed to disk as sorted, immutable SSTable files.

## Components

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


## Contributing

Contributions are welcome! Since this is an early-stage project, please open an issue to discuss major changes before implementing them.

## License

[MIT](LICENSE)
