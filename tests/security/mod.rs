pub mod cryptography;

pub use cryptography::run_crypto_tests;

/// Run all security tests
pub fn run_security_tests() {
    run_crypto_tests();
} 