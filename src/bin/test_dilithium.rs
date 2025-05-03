use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage, DetachedSignature};
use rand::{rngs::OsRng, RngCore};

fn main() {
    println!("Testing Dilithium signatures...");
    
    // Generate random message
    let message = random_message(100);
    println!("Generated random message of length 100");
    
    // Generate keypair
    println!("Generating keypair...");
    let (pk, sk) = dilithium3::keypair();
    
    println!("Public key size: {} bytes", pk.as_bytes().len());
    println!("Secret key size: {} bytes", sk.as_bytes().len());
    
    // Sign the message
    println!("Signing message...");
    let signature = dilithium3::detached_sign(&message, &sk);
    println!("Signature size: {} bytes", signature.as_bytes().len());
    
    // Verify the signature
    println!("Verifying signature...");
    let result = dilithium3::verify_detached_signature(&signature, &message, &pk);
    
    match result {
        Ok(()) => println!("✅ Verification successful!"),
        Err(_) => println!("❌ Verification failed!"),
    }
    
    // Try to tamper with the message
    let mut tampered_message = message.clone();
    if !tampered_message.is_empty() {
        tampered_message[0] ^= 1; // Flip a bit
        println!("\nVerifying with tampered message...");
        let tampered_result = dilithium3::verify_detached_signature(&signature, &tampered_message, &pk);
        
        match tampered_result {
            Ok(()) => println!("✅ Verification successful (should have failed!)"),
            Err(_) => println!("❌ Verification failed (as expected)"),
        }
    }
}

fn random_message(len: usize) -> Vec<u8> {
    let mut message = vec![0u8; len];
    OsRng.fill_bytes(&mut message);
    message
} 