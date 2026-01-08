use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

//SSTABLE WRITER

pub struct SSTableWriter {
    writer: BufWriter<File>,
}

impl SSTableWriter {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_entry(&mut self, key: &[u8], value: &Option<Vec<u8>>) -> std::io::Result<()> {
        // write key length
        let key_len = key.len() as u32;
        self.writer.write_all(&key_len.to_le_bytes())?;
        self.writer.write_all(key)?;

        match value {
            Some(v) => {
                let val_len = v.len() as i32;
                self.writer.write_all(&val_len.to_le_bytes())?;
                self.writer.write_all(v)?;
            }
            None => {
                // tombstone
                let tombstone: i32 = -1;
                self.writer.write_all(&tombstone.to_le_bytes())?;
            }
        }

        Ok(())
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

pub struct SSTableReader {
    reader: BufReader<File>,
}

impl SSTableReader {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    /// Returns:
    /// - Ok(None)            => key not found in this SSTable
    /// - Ok(Some(None))      => tombstone (deleted)
    /// - Ok(Some(Some(v)))  => value found
    pub fn get(&mut self, target_key: &[u8]) -> std::io::Result<Option<Option<Vec<u8>>>> {
        loop {
            // read key length
            let mut key_len_buf = [0u8; 4];

            // EOF reached
            if self.reader.read_exact(&mut key_len_buf).is_err() {
                return Ok(None);
            }

            let key_len = u32::from_le_bytes(key_len_buf) as usize;

            // read key
            let mut key = vec![0u8; key_len];
            self.reader.read_exact(&mut key)?;

            // read value length
            let mut val_len_buf = [0u8; 4];
            self.reader.read_exact(&mut val_len_buf)?;
            let val_len = i32::from_le_bytes(val_len_buf);

            // key match
            if key == target_key {
                if val_len == -1 {
                    // tombstone
                    return Ok(Some(None));
                }

                let mut value = vec![0u8; val_len as usize];
                self.reader.read_exact(&mut value)?;
                return Ok(Some(Some(value)));
            }

            // skip value if key does not match
            if val_len > 0 {
                self.reader
                    .by_ref()
                    .take(val_len as u64)
                    .read_to_end(&mut Vec::new())?;
            }
        }
    }
}
