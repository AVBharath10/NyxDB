use crate::memtable::memtable::MemTable;
use crate::wal::wal::Wal;
use std::path::Path;

pub fn recover<P: AsRef<Path>>(path: P) -> std::io::Result<MemTable> {
    let records = Wal::read_all(path)?;
    let mut memtable = MemTable::new();
    for record in records {
        memtable.apply(&record)?;
    }
    Ok(memtable)
}
