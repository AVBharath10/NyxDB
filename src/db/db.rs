use crate::memtable::memtable::MemTable;
use crate::recov::recovery::recover;
use crate::sstable::sstable::{SSTableReader, SSTableWriter};
use crate::wal::wal::Wal;

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

const MEMTABLE_MAX_ENTRIES: usize = 1000;

pub struct NyxDB {
    wal: Wal,
    memtable: MemTable,
    next_sstable_id: u64,
    data_dir: PathBuf,
    wal_path: PathBuf,
}

impl NyxDB {
    pub fn open<P: AsRef<Path>>(root: P) -> std::io::Result<Self> {
        let root = root.as_ref();
        std::fs::create_dir_all(root)?;

        let sstable_dir = root.join("sstables");
        std::fs::create_dir_all(&sstable_dir)?;

        let wal_path = root.join("wal.log");

        // Recover MemTable
        let memtable = recover(&wal_path)?;

        // Open WAL
        let wal = Wal::open(&wal_path)?;

        // Recover next SSTable ID
        let mut next_sstable_id = 0;
        for entry in std::fs::read_dir(&sstable_dir)? {
            let entry = entry?;
            if let Some(stem) = entry.path().file_stem() {
                if let Ok(id) = stem.to_string_lossy().parse::<u64>() {
                    next_sstable_id = next_sstable_id.max(id + 1);
                }
            }
        }

        Ok(Self {
            wal,
            memtable,
            next_sstable_id,
            data_dir: sstable_dir,
            wal_path,
        })
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::io::Result<()> {
        let mut record = Vec::new();

        record.push(1); // PUT
        record.extend_from_slice(&(key.len() as u32).to_le_bytes());
        record.extend_from_slice(&key);
        record.extend_from_slice(&(value.len() as u32).to_le_bytes());
        record.extend_from_slice(&value);

        self.wal.append(&record)?;
        self.wal.sync()?;

        self.memtable.put(key, value);

        if self.memtable.len() >= MEMTABLE_MAX_ENTRIES {
            self.flush_memtable()?;
        }

        Ok(())
    }

    pub fn delete(&mut self, key: Vec<u8>) -> std::io::Result<()> {
        let mut record = Vec::new();

        record.push(2); // DELETE
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

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if let Some(v) = self.memtable.get(key) {
            return Some(v.clone());
        }

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

    pub fn compact(&mut self) -> std::io::Result<()> {
        // Collect all SSTables (newest → oldest)
        let mut paths = Vec::new();
        for id in (0..self.next_sstable_id).rev() {
            let path = self.data_dir.join(format!("{:06}.sst", id));
            if path.exists() {
                paths.push(path);
            }
        }

        if paths.len() <= 1 {
            return Ok(());
        }

        // Merge map (newest wins)
        let mut merged: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();

        for path in paths.iter() {
            let mut reader = SSTableReader::open(path)?;
            for (key, value) in reader.iter_all()? {
                if !merged.contains_key(&key) {
                    merged.insert(key, value);
                }
            }
        }

        // Drop tombstones (safe: all SSTables compacted)
        merged.retain(|_, v| v.is_some());

        // Write compacted SSTable
        let tmp_path = self.data_dir.join("compaction.tmp");
        let mut writer = SSTableWriter::create(&tmp_path)?;

        for (key, value) in merged {
            writer.write_entry(&key, &value)?;
        }

        writer.finish()?;

        // Delete old SSTables FIRST (simpler v1 safety)
        for id in 0..self.next_sstable_id {
            let path = self.data_dir.join(format!("{:06}.sst", id));
            let _ = std::fs::remove_file(path);
        }

        // Install new SSTable
        let final_path = self.data_dir.join("000000.sst");
        std::fs::rename(tmp_path, final_path)?;

        self.next_sstable_id = 1;

        // Reset WAL (new baseline)
        File::create(&self.wal_path)?;
        self.wal = Wal::open(&self.wal_path)?;

        Ok(())
    }
}
