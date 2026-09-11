use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

const INDEX_INTERVAL: usize = 16;

const FOOTER_LEN: u64 = 16;

pub struct SSTableWriter {
    writer: BufWriter<File>,
    offset: u64,
    entry_count: usize,
    sparse_index: Vec<(Vec<u8>, u64)>,
}

impl SSTableWriter {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            offset: 0,
            entry_count: 0,
            sparse_index: Vec::new(),
        })
    }

    pub fn write_entry(&mut self, key: &[u8], value: &Option<Vec<u8>>) -> std::io::Result<()> {
        
        if self.entry_count % INDEX_INTERVAL == 0 {
            self.sparse_index.push((key.to_vec(), self.offset));
        }

        let key_len = key.len() as u32;
        self.writer.write_all(&key_len.to_le_bytes())?;
        self.writer.write_all(key)?;
        self.offset += 4 + key.len() as u64;

        match value {
            Some(v) => {
                let val_len = v.len() as i32;
                self.writer.write_all(&val_len.to_le_bytes())?;
                self.writer.write_all(v)?;
                self.offset += 4 + v.len() as u64;
            }
            None => {
                let tombstone: i32 = -1;
                self.writer.write_all(&tombstone.to_le_bytes())?;
                self.offset += 4;
            }
        }

        self.entry_count += 1;
        Ok(())
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        let index_start = self.offset;

        for (key, entry_offset) in &self.sparse_index {
            let key_len = key.len() as u32;
            self.writer.write_all(&key_len.to_le_bytes())?;
            self.writer.write_all(key)?;
            self.writer.write_all(&entry_offset.to_le_bytes())?;
        }

        let index_count = self.sparse_index.len() as u64;
        self.writer.write_all(&index_start.to_le_bytes())?;
        self.writer.write_all(&index_count.to_le_bytes())?;

        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }
}

pub struct SSTableReader {
    file: BufReader<File>,
    /// Byte offset where the data block ends (and the index block begins).
    data_end: u64,
    sparse_index: Vec<(Vec<u8>, u64)>,
}

impl SSTableReader {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let mut file = BufReader::new(File::open(path)?);
        let file_len = file.get_ref().metadata()?.len();

        file.seek(SeekFrom::Start(file_len - FOOTER_LEN))?;
        let mut footer = [0u8; FOOTER_LEN as usize];
        file.read_exact(&mut footer)?;
        let index_start = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        let index_count = u64::from_le_bytes(footer[8..16].try_into().unwrap());

        file.seek(SeekFrom::Start(index_start))?;
        let mut sparse_index = Vec::with_capacity(index_count as usize);
        for _ in 0..index_count {
            let mut key_len_buf = [0u8; 4];
            file.read_exact(&mut key_len_buf)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;

            let mut key = vec![0u8; key_len];
            file.read_exact(&mut key)?;

            let mut offset_buf = [0u8; 8];
            file.read_exact(&mut offset_buf)?;
            let offset = u64::from_le_bytes(offset_buf);

            sparse_index.push((key, offset));
        }

        Ok(Self {
            file,
            data_end: index_start,
            sparse_index,
        })
    }

    pub fn iter_all(&mut self) -> std::io::Result<Vec<(Vec<u8>, Option<Vec<u8>>)>> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut pos: u64 = 0;
        let mut entries = Vec::new();

        while pos < self.data_end {
            let mut key_len_buf = [0u8; 4];
            self.file.read_exact(&mut key_len_buf)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;
            let mut key = vec![0u8; key_len];
            self.file.read_exact(&mut key)?;

            let mut val_len_buf = [0u8; 4];
            self.file.read_exact(&mut val_len_buf)?;
            let val_len = i32::from_le_bytes(val_len_buf);
            pos += 4 + key_len as u64 + 4;

            let value = if val_len == -1 {
                None
            } else {
                let mut v = vec![0u8; val_len as usize];
                self.file.read_exact(&mut v)?;
                pos += val_len as u64;
                Some(v)
            };

            entries.push((key, value));
        }

        Ok(entries)
    }

    pub fn get(&mut self, target_key: &[u8]) -> std::io::Result<Option<Option<Vec<u8>>>> {
        let mut pos = self.find_start_offset(target_key);
        self.file.seek(SeekFrom::Start(pos))?;

        while pos < self.data_end {
            let mut key_len_buf = [0u8; 4];
            self.file.read_exact(&mut key_len_buf)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;

            let mut key = vec![0u8; key_len];
            self.file.read_exact(&mut key)?;

            let mut val_len_buf = [0u8; 4];
            self.file.read_exact(&mut val_len_buf)?;
            let val_len = i32::from_le_bytes(val_len_buf);
            pos += 4 + key_len as u64 + 4;

            if key.as_slice() == target_key {
                if val_len == -1 {
                    return Ok(Some(None));
                }
                let mut value = vec![0u8; val_len as usize];
                self.file.read_exact(&mut value)?;
                return Ok(Some(Some(value)));
            }
            if key.as_slice() > target_key {
                return Ok(None);
            }

            if val_len > 0 {
                self.file.seek(SeekFrom::Current(val_len as i64))?;
            }
            pos += val_len.max(0) as u64;
        }

        Ok(None)
    }

    fn find_start_offset(&self, target_key: &[u8]) -> u64 {
        match self
            .sparse_index
            .partition_point(|(key, _)| key.as_slice() <= target_key)
        {
            0 => 0,
            n => self.sparse_index[n - 1].1,
        }
    }
}
