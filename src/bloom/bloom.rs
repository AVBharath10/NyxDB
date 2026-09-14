use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A bit array ("boxes") plus a count of how many boxes each key touches.
///
/// `might_contain` can only ever say "definitely not" or "maybe" — it must
/// never say "definitely not" for a key that was actually inserted.
/// False positives ("maybe" for a key never inserted) are allowed.
pub struct BloomFilter {
    bits: Vec<bool>,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(num_bits: usize, num_hashes: usize) -> Self {
        Self {
            bits: vec![false; num_bits.max(1)],
            num_hashes: num_hashes.max(1),
        }
    }

    pub fn insert(&mut self, key: &[u8]) {
        for i in 0..self.num_hashes {
            let idx = self.bit_index(key, i);
            self.bits[idx] = true;
        }
    }

    pub fn might_contain(&self, key: &[u8]) -> bool {
        (0..self.num_hashes).all(|i| self.bits[self.bit_index(key, i)])
    }

    /// Picks the i-th "box" for a key. Rather than needing `num_hashes`
    /// truly independent hash functions, we hash the key twice and combine
    /// the two results differently for each i (a standard bloom filter
    /// trick, sometimes called double hashing).
    fn bit_index(&self, key: &[u8], i: usize) -> usize {
        let h1 = hash_with_seed(key, 0);
        let h2 = hash_with_seed(key, 1);
        let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
        (combined % self.bits.len() as u64) as usize
    }
}

fn hash_with_seed(key: &[u8], seed: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    key.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::BloomFilter;

    #[test]
    fn definitely_not_present_for_a_key_never_inserted() {
        let mut filter = BloomFilter::new(1000, 3);
        filter.insert(b"cat");
        filter.insert(b"dog");

        assert!(!filter.might_contain(b"fox"));
    }

    #[test]
    fn probably_present_for_an_inserted_key() {
        let mut filter = BloomFilter::new(1000, 3);
        filter.insert(b"cat");

        assert!(filter.might_contain(b"cat"));
    }

    #[test]
    fn never_false_negative_across_many_keys() {
        let mut filter = BloomFilter::new(10_000, 5);
        let keys: Vec<String> = (0..2000).map(|i| format!("key-{i}")).collect();

        for k in &keys {
            filter.insert(k.as_bytes());
        }

        // Every inserted key must always come back "maybe" (never "no").
        for k in &keys {
            assert!(filter.might_contain(k.as_bytes()));
        }
    }
}
