use crate::types::Hash;
use blake3;

/// MerkleNode represents a node in a Merkle tree
#[derive(Debug, Clone)]
pub struct MerkleNode {
    /// The hash of this node
    pub hash: Hash,
    
    /// Left child (if any)
    pub left: Option<Box<MerkleNode>>,
    
    /// Right child (if any)
    pub right: Option<Box<MerkleNode>>,
}

/// MerkleTree is a tree of hashes for efficient verification
pub struct MerkleTree {
    /// The root node of the tree
    pub root: Option<MerkleNode>,
    
    /// The leaves of the tree
    pub leaves: Vec<Hash>,
}

impl MerkleTree {
    /// Create a new empty Merkle tree
    pub fn new() -> Self {
        Self {
            root: None,
            leaves: Vec::new(),
        }
    }
    
    /// Create a Merkle tree from a set of leaves
    pub fn from_leaves(leaves: Vec<Hash>) -> Self {
        let mut tree = Self {
            root: None,
            leaves: leaves.clone(),
        };
        
        tree.build();
        tree
    }
    
    /// Build the Merkle tree from the leaves
    pub fn build(&mut self) {
        if self.leaves.is_empty() {
            self.root = None;
            return;
        }
        
        // If there's just one leaf, it's also the root
        if self.leaves.len() == 1 {
            self.root = Some(MerkleNode {
                hash: self.leaves[0],
                left: None,
                right: None,
            });
            return;
        }
        
        // Build the tree from bottom up
        let mut nodes = self.leaves.iter().map(|leaf| MerkleNode {
            hash: *leaf,
            left: None,
            right: None,
        }).collect::<Vec<_>>();
        
        // Keep combining nodes until we have a single root
        while nodes.len() > 1 {
            let mut next_level = Vec::new();
            
            // Process pairs of nodes
            for chunk in nodes.chunks(2) {
                let left = chunk[0].clone();
                let right = if chunk.len() > 1 { 
                    chunk[1].clone() 
                } else { 
                    left.clone() // Duplicate the last node if odd
                };
                
                // Combine hashes to create parent
                let mut combined = Vec::with_capacity(64);
                combined.extend_from_slice(&left.hash);
                combined.extend_from_slice(&right.hash);
                
                let hash = {
                    let hash_result = blake3::hash(&combined);
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(hash_result.as_bytes());
                    hash
                };
                
                // Create parent node
                let parent = MerkleNode {
                    hash,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                };
                
                next_level.push(parent);
            }
            
            nodes = next_level;
        }
        
        // Set the root
        self.root = nodes.into_iter().next();
    }
    
    /// Get the root hash of the tree
    pub fn root_hash(&self) -> Option<Hash> {
        self.root.as_ref().map(|node| node.hash)
    }
    
    /// Create a proof for a specific leaf index
    pub fn create_proof(&self, leaf_index: usize) -> Option<Vec<Hash>> {
        if leaf_index >= self.leaves.len() {
            return None;
        }
        
        let mut proof = Vec::new();
        let mut index = leaf_index;
        let mut level = self.leaves.len();
        let mut current = self.root.as_ref()?;
        
        // Traverse the tree to build the proof
        while level > 1 {
            let sibling_idx = index ^ 1; // XOR with 1 to get sibling index
            let sibling = self.get_sibling(current, index % 2 == 0)?;
            proof.push(sibling);
            
            // Move up one level
            level = (level + 1) / 2;
            index /= 2;
            
            if let Some(ref next) = if index % 2 == 0 { 
                current.left.as_ref() 
            } else { 
                current.right.as_ref() 
            } {
                current = next;
            } else {
                break;
            }
        }
        
        Some(proof)
    }
    
    /// Get the sibling hash of a node
    fn get_sibling(&self, node: &MerkleNode, is_left: bool) -> Option<Hash> {
        if is_left {
            node.right.as_ref().map(|n| n.hash)
        } else {
            node.left.as_ref().map(|n| n.hash)
        }
    }
}

/// Verify a Merkle proof
pub fn verify_proof(root_hash: &Hash, leaf_hash: &Hash, proof: &[Hash], leaf_index: usize) -> bool {
    let mut current_hash = *leaf_hash;
    let mut index = leaf_index;
    
    for sibling in proof {
        // Combine current hash with sibling based on position
        let mut combined = Vec::with_capacity(64);
        
        if index % 2 == 0 {
            // Current is left, sibling is right
            combined.extend_from_slice(&current_hash);
            combined.extend_from_slice(sibling);
        } else {
            // Current is right, sibling is left
            combined.extend_from_slice(sibling);
            combined.extend_from_slice(&current_hash);
        }
        
        // Hash the combined data
        let hash_result = blake3::hash(&combined);
        current_hash = {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(hash_result.as_bytes());
            hash
        };
        
        // Move up one level
        index /= 2;
    }
    
    // Check if the computed hash matches the root hash
    &current_hash == root_hash
}
