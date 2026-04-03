use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupResult {
    Present,
    Deleted,
    Absent,
}

#[derive(Debug, Default)]
pub struct MemTable {
    map: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl MemTable {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Option<Vec<u8>>)> {
        self.map.iter()
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.map.insert(key, Some(value));
    }

    pub fn lookup(&self, key: &[u8]) -> LookupResult {
        match self.map.get(key) {
            Some(Some(_)) => LookupResult::Present,
            Some(None) => LookupResult::Deleted,
            None => LookupResult::Absent,
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.map.get(key)?.as_ref()
    }

    pub fn delete(&mut self, key: Vec<u8>) {
        self.map.insert(key, None);
    }

    pub fn apply(&mut self, record: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;

        let op = *record.get(offset).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "WAL record missing opcode")
        })?;
        offset += 1;

        let key_len = read_u32(record, &mut offset)? as usize;

        if offset + key_len > record.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "WAL record key truncated",
            ));
        }

        let key = record[offset..offset + key_len].to_vec();
        offset += key_len;

        match op {
            1 => {
                let val_len = read_u32(record, &mut offset)? as usize;

                if offset + val_len > record.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "WAL record value truncated",
                    ));
                }
                let value = record[offset..offset + val_len].to_vec();
                self.put(key, value);
            }

            2 => {
                self.delete(key);
            }

            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unknown WAL operation",
                ));
            }
        }

        Ok(())
    }
}

fn read_u32(record: &[u8], offset: &mut usize) -> std::io::Result<u32> {
    if *offset + 4 > record.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "WAL record truncated",
        ));
    }

    let value = u32::from_le_bytes(
        record[*offset..*offset + 4]
            .try_into()
            .expect("slice length checked above"),
    );
    *offset += 4;
    Ok(value)
}
