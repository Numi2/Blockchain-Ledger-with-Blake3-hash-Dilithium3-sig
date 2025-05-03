// This file is a simple test of the Dilithium signatures implementation
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage, DetachedSignature};
use rand::{rngs::OsRng, RngCore};

/// Generate a random message
fn random_message(len: usize) -> Vec<u8> {
    let mut message = vec![0u8; len];
    OsRng.fill_bytes(&mut message);
    message
}

/// Test the dilithium3 signature scheme
pub fn test_dilithium3() -> Result<(), String> {
    // Generate a random message
    let message = random_message(100);
    
    // Generate a keypair
    let (pk, sk) = dilithium3::keypair();
    
    // Sign the message
    let signature = dilithium3::detached_sign(&message, &sk);
    
    // Verify the signature
    let result = dilithium3::verify_detached_signature(&signature, &message, &pk);
    
    if result.is_ok() {
        Ok(())
    } else {
        Err("Signature verification failed".to_string())
    }
}

/// Run a full test of the signature scheme
pub fn run_test() {
    match test_dilithium3() {
        Ok(_) => println!("Dilithium3 signature test passed!"),
        Err(e) => println!("Dilithium3 signature test failed: {}", e),
    }
} 