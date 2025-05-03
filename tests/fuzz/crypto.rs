// Fuzzing tests for cryptographic components to ensure security

use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, DetachedSignature};
use blake3::{Hash, Hasher};
use rand::{Rng, rngs::OsRng, RngCore};

/// Generate a random message for testing
fn random_message(len: usize) -> Vec<u8> {
    let mut message = vec![0u8; len];
    OsRng.fill_bytes(&mut message);
    message
}

/// Fuzz test for Dilithium3 signatures
#[cfg(test)]
mod fuzz_dilithium {
    use super::*;
    
    /// Test signature generation and verification with random messages
    #[test]
    fn test_dilithium3_random_messages() {
        // Generate a keypair
        let (pk, sk) = dilithium3::keypair();
        
        // Test with various message sizes
        for size in [0, 1, 10, 100, 1000, 10000].iter() {
            let message = random_message(*size);
            
            // Sign the message
            let signature = dilithium3::detached_sign(&message, &sk);
            
            // Verify the signature
            let result = dilithium3::verify_detached_signature(&signature, &message, &pk);
            assert!(result.is_ok(), "Verification failed for message of size {}", size);
        }
    }
    
    /// Test signature verification with corrupted signatures
    #[test]
    fn test_dilithium3_corrupted_signatures() {
        // Generate a keypair
        let (pk, sk) = dilithium3::keypair();
        
        // Create a test message
        let message = random_message(100);
        
        // Sign the message
        let signature = dilithium3::detached_sign(&message, &sk);
        let sig_bytes = signature.as_bytes();
        
        // Test with corrupted signatures
        for i in 0..10 {
            // Create a corrupted signature by modifying a random byte
            let mut corrupted_sig_bytes = sig_bytes.to_vec();
            let idx = rand::thread_rng().gen_range(0..corrupted_sig_bytes.len());
            corrupted_sig_bytes[idx] ^= 0xff; // Flip all bits in this byte
            
            // Create signature from corrupted bytes
            let corrupted_sig = match dilithium3::DetachedSignature::from_bytes(&corrupted_sig_bytes) {
                Ok(sig) => sig,
                Err(_) => continue, // Skip if corrupted signature is not valid format
            };
            
            // Verify the corrupted signature (should fail)
            let result = dilithium3::verify_detached_signature(&corrupted_sig, &message, &pk);
            assert!(result.is_err(), "Verification succeeded with corrupted signature (iteration {})", i);
        }
    }
    
    /// Test signature verification with wrong messages
    #[test]
    fn test_dilithium3_wrong_messages() {
        // Generate a keypair
        let (pk, sk) = dilithium3::keypair();
        
        // Create a test message and sign it
        let message = random_message(100);
        let signature = dilithium3::detached_sign(&message, &sk);
        
        // Test with modified messages
        for i in 0..10 {
            // Create a modified message
            let mut wrong_message = message.clone();
            let idx = rand::thread_rng().gen_range(0..wrong_message.len());
            wrong_message[idx] ^= 0xff; // Flip all bits in this byte
            
            // Verify with wrong message
            let result = dilithium3::verify_detached_signature(&signature, &wrong_message, &pk);
            assert!(result.is_err(), "Verification succeeded with wrong message (iteration {})", i);
        }
    }
    
    /// Test signature verification with wrong public keys
    #[test]
    fn test_dilithium3_wrong_public_keys() {
        // Create a test message
        let message = random_message(100);
        
        // Generate multiple keypairs
        let (pk1, sk1) = dilithium3::keypair();
        let (pk2, _) = dilithium3::keypair();
        
        // Sign with first key
        let signature = dilithium3::detached_sign(&message, &sk1);
        
        // Verify with second key (should fail)
        let result = dilithium3::verify_detached_signature(&signature, &message, &pk2);
        assert!(result.is_err(), "Verification succeeded with wrong public key");
    }
    
    /// Test with many different keypairs
    #[test]
    fn test_dilithium3_multiple_keypairs() {
        const NUM_KEYPAIRS: usize = 10;
        
        for _ in 0..NUM_KEYPAIRS {
            // Generate a new keypair
            let (pk, sk) = dilithium3::keypair();
            
            // Create a test message
            let message = random_message(100);
            
            // Sign and verify
            let signature = dilithium3::detached_sign(&message, &sk);
            let result = dilithium3::verify_detached_signature(&signature, &message, &pk);
            
            assert!(result.is_ok(), "Verification failed with fresh keypair");
        }
    }
}

/// Fuzz test for BLAKE3 hash function
#[cfg(test)]
mod fuzz_blake3 {
    use super::*;
    
    /// Test BLAKE3 hashing with different message sizes
    #[test]
    fn test_blake3_different_sizes() {
        // Test with various message sizes
        for size in [0, 1, 10, 63, 64, 65, 127, 128, 129, 1000, 10000].iter() {
            let message = random_message(*size);
            
            // Hash the message
            let hash = blake3::hash(&message);
            
            // Hash should always be 32 bytes
            assert_eq!(hash.as_bytes().len(), 32);
        }
    }
    
    /// Test BLAKE3 with incremental updates
    #[test]
    fn test_blake3_incremental() {
        for _ in 0..10 {
            // Create random message
            let message = random_message(1000);
            
            // Hash in one go
            let hash1 = blake3::hash(&message);
            
            // Hash incrementally
            let mut hasher = Hasher::new();
            
            // Split message into random chunks
            let mut pos = 0;
            while pos < message.len() {
                let chunk_size = rand::thread_rng().gen_range(1..=64.min(message.len() - pos));
                hasher.update(&message[pos..pos + chunk_size]);
                pos += chunk_size;
            }
            
            let hash2 = hasher.finalize();
            
            // Both methods should produce the same hash
            assert_eq!(hash1.as_bytes(), hash2.as_bytes());
        }
    }
    
    /// Test BLAKE3 collision resistance with similar messages
    #[test]
    fn test_blake3_similar_messages() {
        // Create base message
        let base_message = random_message(100);
        
        // Hash base message
        let base_hash = blake3::hash(&base_message);
        
        // Test with modified messages
        for i in 0..100 {
            // Create a modified message (change a single bit)
            let mut modified_message = base_message.clone();
            let byte_idx = i % modified_message.len();
            let bit_idx = (i / modified_message.len()) % 8;
            modified_message[byte_idx] ^= 1 << bit_idx; // Flip a single bit
            
            // Hash the modified message
            let modified_hash = blake3::hash(&modified_message);
            
            // Hashes should be different
            assert_ne!(
                base_hash.as_bytes(), 
                modified_hash.as_bytes(),
                "Hash collision found for base message and message with bit {} of byte {} flipped",
                bit_idx, byte_idx
            );
        }
    }
    
    /// Test BLAKE3 hash with zero bytes
    #[test]
    fn test_blake3_zero_bytes() {
        // Hash all zeros
        let zeros = vec![0u8; 1000];
        let hash1 = blake3::hash(&zeros);
        
        // Hash all zeros again (should be same)
        let hash2 = blake3::hash(&zeros);
        
        assert_eq!(hash1.as_bytes(), hash2.as_bytes());
        
        // Hash a different message
        let ones = vec![1u8; 1000];
        let hash3 = blake3::hash(&ones);
        
        // Should be different
        assert_ne!(hash1.as_bytes(), hash3.as_bytes());
    }
    
    /// Test with extremely large input
    #[test]
    fn test_blake3_large_input() {
        // 1 MB of random data
        let large_input = random_message(1024 * 1024);
        
        // Hash it
        let hash = blake3::hash(&large_input);
        
        // Hash should still be 32 bytes
        assert_eq!(hash.as_bytes().len(), 32);
    }
}

/// Extended security tests combining multiple cryptographic primitives
#[cfg(test)]
mod security_tests {
    use super::*;
    use std::collections::HashSet;
    
    /// Test hash uniqueness for many random inputs
    #[test]
    fn test_hash_uniqueness() {
        const NUM_HASHES: usize = 1000;
        let mut hashes = HashSet::with_capacity(NUM_HASHES);
        
        for _ in 0..NUM_HASHES {
            let message = random_message(100);
            let hash = blake3::hash(&message);
            let hash_bytes = hash.as_bytes().to_vec();
            
            // Should not have seen this hash before
            assert!(hashes.insert(hash_bytes), "Hash collision detected");
        }
        
        // We should have NUM_HASHES unique hashes
        assert_eq!(hashes.len(), NUM_HASHES);
    }
    
    /// Test signing multiple messages with same key
    #[test]
    fn test_multiple_signatures_same_key() {
        const NUM_SIGNATURES: usize = 100;
        
        // Generate a keypair
        let (pk, sk) = dilithium3::keypair();
        
        for _ in 0..NUM_SIGNATURES {
            // Create a test message and hash it
            let message = random_message(100);
            let message_hash = blake3::hash(&message);
            
            // Sign the hash
            let signature = dilithium3::detached_sign(message_hash.as_bytes(), &sk);
            
            // Verify the signature
            let result = dilithium3::verify_detached_signature(&signature, message_hash.as_bytes(), &pk);
            assert!(result.is_ok(), "Verification failed for hash signature");
        }
    }
    
    /// Test storing signatures in a hash map
    #[test]
    fn test_signatures_in_hashmap() {
        const NUM_MESSAGES: usize = 100;
        
        // Generate a keypair
        let (pk, sk) = dilithium3::keypair();
        
        // Create a map of message hash -> signature
        let mut signatures = HashMap::with_capacity(NUM_MESSAGES);
        
        // Sign multiple messages
        for _ in 0..NUM_MESSAGES {
            let message = random_message(100);
            let message_hash = blake3::hash(&message);
            
            // Sign the hash
            let signature = dilithium3::detached_sign(message_hash.as_bytes(), &sk);
            signatures.insert(message_hash.as_bytes().to_vec(), signature);
        }
        
        // Verify all signatures
        for (hash, signature) in signatures {
            let result = dilithium3::verify_detached_signature(&signature, &hash, &pk);
            assert!(result.is_ok(), "Verification failed for stored signature");
        }
    }
}

/// Run all fuzz tests
pub fn run_all_fuzz_tests() {
    use fuzz_dilithium::*;
    use fuzz_blake3::*;
    use security_tests::*;
    
    // Dilithium3 tests
    test_dilithium3_random_messages();
    test_dilithium3_corrupted_signatures();
    test_dilithium3_wrong_messages();
    test_dilithium3_wrong_public_keys();
    test_dilithium3_multiple_keypairs();
    
    // BLAKE3 tests
    test_blake3_different_sizes();
    test_blake3_incremental();
    test_blake3_similar_messages();
    test_blake3_zero_bytes();
    test_blake3_large_input();
    
    // Combined security tests
    test_hash_uniqueness();
    test_multiple_signatures_same_key();
    test_signatures_in_hashmap();
    
    println!("All cryptographic fuzz tests passed!");
}
