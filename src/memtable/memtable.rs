use std::collections::BTreeMap;

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

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.map.get(key)?.as_ref()
    }

    pub fn delete(&mut self, key: Vec<u8>) {
        self.map.insert(key, None);
    }
    pub fn apply(&mut self, record: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;

        // 1. Read operation
        let op = record[offset];
        offset += 1;

        // 2. Read key length
        let key_len = u32::from_le_bytes(record[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        // 3. Read key
        let key = record[offset..offset + key_len].to_vec();
        offset += key_len;

        match op {
            // PUT
            1 => {
                let val_len =
                    u32::from_le_bytes(record[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;

                let value = record[offset..offset + val_len].to_vec();
                self.put(key, value);
            }

            // DELETE
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
