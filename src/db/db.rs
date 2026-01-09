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
    pub fn open<P: AsRef<Path>>(root: P) -> std::io::Result<Self> {
        let root = root.as_ref();

        // 1. Create DB root directory
        std::fs::create_dir_all(root)?;

        // 2. SSTable directory
        let sstable_dir = root.join("sstables");
        std::fs::create_dir_all(&sstable_dir)?;

        // 3. WAL FILE path
        let wal_path = root.join("wal.log");

        // 4. Recover MemTable from WAL
        let memtable = recover(&wal_path)?;

        // 5. Open WAL
        let wal = Wal::open(&wal_path)?;

        // 6. Recover next SSTable ID (VERY IMPORTANT)
        let mut next_sstable_id = 0;
        for entry in std::fs::read_dir(&sstable_dir)? {
            let entry = entry?;
            if let Some(stem) = entry.path().file_stem() {
                if let Some(id) = stem.to_string_lossy().parse::<u64>().ok() {
                    next_sstable_id = next_sstable_id.max(id + 1);
                }
            }
        }

        Ok(Self {
            wal,
            memtable,
            next_sstable_id,
            data_dir: sstable_dir,
        })
    }

      // WRITE PATH

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

      // FLUSH LOGIC

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
