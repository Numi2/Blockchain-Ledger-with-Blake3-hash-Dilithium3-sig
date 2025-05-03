use crate::ssz::{SSZError, SSZResult};
use blake3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serialize a value using SSZ
pub fn serialize<T: Serialize>(value: &T) -> SSZResult<Vec<u8>> {
    // For now, we'll use serde_json as a placeholder for SSZ serialization
    // In a real implementation, you would use proper SSZ encoding
    serde_json::to_vec(value)
        .map_err(|e| SSZError::Serialization(e.to_string()))
}

/// Deserialize a value using SSZ
pub fn deserialize<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> SSZResult<T> {
    // For now, we'll use serde_json as a placeholder for SSZ deserialization
    // In a real implementation, you would use proper SSZ decoding
    serde_json::from_slice(bytes)
        .map_err(|e| SSZError::Deserialization(e.to_string()))
}

/// Calculate the hash tree root of a value
pub fn hash_tree_root<T: Serialize>(value: &T) -> SSZResult<[u8; 32]> {
    // For now, we'll use a simplified version that just hashes the serialized bytes
    // In a real implementation, you would build a proper Merkle tree
    let bytes = serialize(value)?;
    let hash = blake3::hash(&bytes);
    
    let mut root = [0u8; 32];
    root.copy_from_slice(hash.as_bytes());
    
    Ok(root)
}

/// Calculate a simple Merkle root from a list of hashes
pub fn merkleize(chunks: &[Vec<u8>]) -> Vec<u8> {
    if chunks.is_empty() {
        return vec![0; 32]; // Return zeroed hash for empty list
    }
    
    if chunks.len() == 1 {
        return chunks[0].clone(); // Single chunk is its own root
    }
    
    // Create chunks of 32 bytes
    let mut padded_chunks: Vec<Vec<u8>> = chunks
        .iter()
        .map(|chunk| {
            let mut padded = chunk.clone();
            if padded.len() < 32 {
                padded.resize(32, 0);
            }
            padded
        })
        .collect();
    
    // Ensure power of 2 chunks by padding
    let next_power_of_2 = 1 << (padded_chunks.len() as f64).log2().ceil() as usize;
    while padded_chunks.len() < next_power_of_2 {
        padded_chunks.push(vec![0; 32]);
    }
    
    // Iteratively hash pairs of chunks to build tree
    while padded_chunks.len() > 1 {
        let mut next_level = Vec::new();
        
        for i in (0..padded_chunks.len()).step_by(2) {
            let left = &padded_chunks[i];
            let right = if i + 1 < padded_chunks.len() {
                &padded_chunks[i + 1]
            } else {
                left // Duplicate the left node if no right exists
            };
            
            // Concatenate and hash the pair
            let mut combined = Vec::with_capacity(left.len() + right.len());
            combined.extend_from_slice(left);
            combined.extend_from_slice(right);
            
            let hash = blake3::hash(&combined);
            next_level.push(hash.as_bytes().to_vec());
        }
        
        padded_chunks = next_level;
    }
    
    padded_chunks[0].clone()
}

/// Create a simple Merkle proof for a specific index in a set of values
pub fn create_proof(chunks: &[Vec<u8>], index: usize) -> Vec<Vec<u8>> {
    if chunks.is_empty() || index >= chunks.len() {
        return Vec::new();
    }
    
    // Create padded chunks
    let mut padded_chunks: Vec<Vec<u8>> = chunks
        .iter()
        .map(|chunk| {
            let mut padded = chunk.clone();
            if padded.len() < 32 {
                padded.resize(32, 0);
            }
            padded
        })
        .collect();
    
    // Ensure power of 2 chunks by padding
    let next_power_of_2 = 1 << (padded_chunks.len() as f64).log2().ceil() as usize;
    while padded_chunks.len() < next_power_of_2 {
        padded_chunks.push(vec![0; 32]);
    }
    
    let mut proof = Vec::new();
    let mut idx = index;
    let mut layer_size = padded_chunks.len();
    let mut layer = padded_chunks;
    
    while layer_size > 1 {
        let sibling_idx = idx ^ 1; // XOR with 1 to get sibling index
        proof.push(layer[sibling_idx].clone());
        
        // Move up one layer in the tree
        let mut next_layer = Vec::new();
        for i in (0..layer_size).step_by(2) {
            let left = &layer[i];
            let right = if i + 1 < layer_size {
                &layer[i + 1]
            } else {
                left
            };
            
            let mut combined = Vec::with_capacity(left.len() + right.len());
            combined.extend_from_slice(left);
            combined.extend_from_slice(right);
            
            let hash = blake3::hash(&combined);
            next_layer.push(hash.as_bytes().to_vec());
        }
        
        layer = next_layer;
        layer_size = layer.len();
        idx /= 2;
    }
    
    proof
}

/// Verify a Merkle proof for a specific value at an index
pub fn verify_proof(root: &[u8], value: &[u8], proof: &[Vec<u8>], index: usize) -> bool {
    let mut padded_value = value.to_vec();
    if padded_value.len() < 32 {
        padded_value.resize(32, 0);
    }
    
    let mut idx = index;
    let mut current = padded_value;
    
    for sibling in proof {
        let (left, right) = if idx % 2 == 0 {
            (&current, sibling)
        } else {
            (sibling, &current)
        };
        
        let mut combined = Vec::with_capacity(left.len() + right.len());
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);
        
        current = blake3::hash(&combined).as_bytes().to_vec();
        idx /= 2;
    }
    
    &current == root
}
