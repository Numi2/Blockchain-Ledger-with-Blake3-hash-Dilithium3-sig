use crate::types::types::Hash;

/// Trait for a Merkle-like binary tree supporting Blake3 hashing.
pub trait MerkleTree {
    /// Insert or update the value at a given key.
    fn insert(&mut self, key: &[u8], value: &[u8]);

    /// Compute and return the current root hash.
    fn root(&self) -> Hash;

    /// Get a proof for a given key. (optional, for future usage)
    fn get_proof(&self, _key: &[u8]) -> Vec<Hash> {
        vec![]
    }
}

/// Merkle tree stub using Blake3 for hashing, production 
field-extendable
pub struct Blake3Merkle {
    // TODO: Replace with efficient map/tree
    pairs: Vec<(Vec<u8>, Vec<u8>)>,
    root: Hash,
}

impl Blake3Merkle {
    pub fn new() -> Self {
        Self { pairs: vec![], root: [0u8; 32] }
    }
}

impl MerkleTree for Blake3Merkle {
    fn insert(&mut self, key: &[u8], value: &[u8]) {
        // This is a stub; use a proper binary tree and hashing strategy
for large-scale
        self.pairs.push((key.to_vec(), value.to_vec()));
        let mut hasher = blake3::Hasher::new(); // Uses blake3 crate
        for (k, v) in &self.pairs {
            hasher.update(k);
            hasher.update(v);
        }
        self.root = *hasher.finalize().as_bytes();
    }

    fn root(&self) -> Hash {
        self.root
    }
}
