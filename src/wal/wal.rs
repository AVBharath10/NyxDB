use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct Wal {
    writer: BufWriter<File>,
}
impl Wal {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer })
    }
    pub fn append(&mut self, data: &[u8]) -> std::io::Result<()> {
        let len = data.len() as u32;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(data)?;
        Ok(())
    }
    pub fn sync(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }
    pub fn read_all<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<Vec<u8>>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];

            match reader.read_exact(&mut len_buf) {
                Ok(_) => {
                    let len = u32::from_le_bytes(len_buf) as usize;
                    let mut data = vec![0u8; len];
                    reader.read_exact(&mut data)?;
                    records.push(data);
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(records)
    }
}
