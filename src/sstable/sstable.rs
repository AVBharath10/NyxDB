use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

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
        // key length
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
