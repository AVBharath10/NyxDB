use crate::memtable::memtable::MemTable;
use crate::recov::recovery::recover;
use crate::sstable::sstable::{SSTableReader, SSTableWriter};
use crate::wal::wal::Wal;
use std::path::{Path, PathBuf};

const MEMTABLE_MAX_ENTRIES: usize = 1000;
pub struct NyxDB {
    wal: Wal,
    memtable: MemTable,
    next_sstable_id: u64,
    data_dir: PathBuf,
}

impl NyxDB {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let data_dir = path.join("sstables");

        std::fs::create_dir_all(&data_dir)?;

        // Recover MemTable from WAL
        let memtable = recover(&path)?;

        // Open WAL for new writes
        let wal = Wal::open(path)?;

        // Determine next SSTable ID
        let mut next_sstable_id = 0;
        if data_dir.exists() {
            for entry in std::fs::read_dir(&data_dir)? {
                let entry = entry?;
                if let Some(name) = entry.path().file_stem() {
                    if let Some(id) = name.to_string_lossy().parse::<u64>().ok() {
                        next_sstable_id = next_sstable_id.max(id + 1);
                    }
                }
            }
        }

        Ok(Self {
            wal,
            memtable,
            next_sstable_id,
            data_dir,
        })
    }

    //WRITE PATH

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::io::Result<()> {
        let mut record = Vec::new();

        // PUT opcode
        record.push(1);
        record.extend_from_slice(&(key.len() as u32).to_le_bytes());
        record.extend_from_slice(&key);
        record.extend_from_slice(&(value.len() as u32).to_le_bytes());
        record.extend_from_slice(&value);

        // WAL first
        self.wal.append(&record)?;
        self.wal.sync()?;

        // MemTable
        self.memtable.put(key, value);

        // Flush if needed
        if self.memtable.len() >= MEMTABLE_MAX_ENTRIES {
            self.flush_memtable()?;
        }

        Ok(())
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

        if self.memtable.len() >= MEMTABLE_MAX_ENTRIES {
            self.flush_memtable()?;
        }

        Ok(())
    }

    //READ PATH

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // 1. MemTable
        if let Some(v) = self.memtable.get(key) {
            return Some(v.clone());
        }

        // 2. SSTables (newest → oldest)
        for id in (0..self.next_sstable_id).rev() {
            let path = self.data_dir.join(format!("{:06}.sst", id));

            if !path.exists() {
                continue;
            }

            let mut reader = SSTableReader::open(&path).ok()?;

            if let Ok(Some(entry)) = reader.get(key) {
                return entry;
            }
        }

        None
    }

    //FLUSH LOGIC

    fn flush_memtable(&mut self) -> std::io::Result<()> {
        let path = self
            .data_dir
            .join(format!("{:06}.sst", self.next_sstable_id));

        let mut writer = SSTableWriter::create(&path)?;

        for (key, value) in self.memtable.iter() {
            writer.write_entry(key, value)?;
        }

        writer.finish()?;

        self.memtable.clear();
        self.next_sstable_id += 1;

        Ok(())
    }
}
