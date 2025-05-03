use crate::fuzz::crypto;

/// Run all cryptographic security tests
pub fn run_crypto_tests() {
    println!("Running cryptographic security tests...");
    crypto::run_all_fuzz_tests();
} 