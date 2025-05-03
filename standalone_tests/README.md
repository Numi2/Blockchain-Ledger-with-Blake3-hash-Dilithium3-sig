# Standalone Tests

This directory contains standalone tests for specific components of the World Ledger blockchain. These tests are independent of the main project and can be run separately to validate the functionality of individual components.

## Encryption Test

The `encryption_test.rs` file demonstrates our implementation of memory-hardened password hashing using Argon2id, combined with AES-GCM encryption. This test shows:

1. Key derivation from a password using Argon2id
2. Encryption of data using the derived key
3. Decryption using the same password
4. Verification that using an incorrect password fails

### Running the Test

```bash
cd standalone_tests
cargo run
```

### Key Security Features

- **Memory-hardening**: Uses Argon2id which is designed to require significant memory, making it resistant to GPU/ASIC attacks
- **Secure key handling**: Uses `secrecy` to protect keys in memory
- **Memory cleanup**: Uses `zeroize` to securely erase sensitive data when no longer needed
- **Constant-time operations**: Implements operations in constant time to prevent timing attacks

### Example Output

```
Data encrypted successfully!
Ciphertext length: 57 bytes
Salt: 6iZ1VK1McBN5+4/LIZv1uw

Decryption successful!
Original: This is a secret message for testing Argon2id encryption
Decrypted: This is a secret message for testing Argon2id encryption

Verification successful: decrypted data matches original!
Test passed: Decryption with wrong password correctly failed
```

## Adding More Tests

To add additional standalone tests:

1. Create a new `.rs` file in this directory
2. Add dependencies to the `Cargo.toml` file
3. Update this README with information about the new test

This approach allows for testing specific components without the complexity of the full blockchain codebase. 