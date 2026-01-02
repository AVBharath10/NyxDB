use crate::memtable::memtable::MemTable;
use crate::recov::recovery::recover;
use crate::wal::wal::Wal;
use std::path::Path;

pub struct NyxDB {
    wal: Wal,
    memtable: MemTable,
}

impl NyxDB {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        // Recover MemTable from WAL
        let memtable = recover(&path)?;

        // Open WAL for new writes
        let wal = Wal::open(path)?;

        Ok(Self { wal, memtable })
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::io::Result<()> {
        let mut record = Vec::new();

        // PUT opcode
        record.push(1);
        record.extend_from_slice(&(key.len() as u32).to_le_bytes());
        record.extend_from_slice(&key);
        record.extend_from_slice(&(value.len() as u32).to_le_bytes());
        record.extend_from_slice(&value);

        // WAL first (durability)
        self.wal.append(&record)?;
        self.wal.sync()?;

        // Then memory
        self.memtable.put(key, value);

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.memtable.get(key)
    }

    pub fn delete(&mut self, key: Vec<u8>) -> std::io::Result<()> {
        let mut record = Vec::new();

        // DELETE opcode
        record.push(2);
        record.extend_from_slice(&(key.len() as u32).to_le_bytes());
        record.extend_from_slice(&key);

        self.wal.append(&record)?;
        self.wal.sync()?;

        self.memtable.delete(key);

        Ok(())
    }
}
