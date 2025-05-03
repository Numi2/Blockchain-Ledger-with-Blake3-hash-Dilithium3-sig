use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2, Params, Algorithm, Version,
};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng as AesOsRng},
    Aes256Gcm, Key, Nonce,
};
use secrecy::{Secret, SecretString, ExposeSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};
use std::fmt;

// Encryption error
#[derive(Debug)]
enum EncryptionError {
    PasswordHash(String),
    Encryption(String),
    Decryption(String),
    InvalidParams(String),
}

// Result type for encryption operations
type EncryptionResult<T> = Result<T, EncryptionError>;

// Encryption key derived from password
#[derive(Clone, ZeroizeOnDrop)]
struct DerivedKey {
    // The key bytes
    #[zeroize(skip)]
    key: Secret<Vec<u8>>,
}

impl fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl DerivedKey {
    // Get the key bytes
    pub fn expose_key(&self) -> &[u8] {
        self.key.expose_secret()
    }
}

// Key derivation parameters
#[derive(Debug, Clone)]
struct KdfParams {
    // Memory cost (in KB)
    memory_cost: u32,
    
    // Time cost (iterations)
    time_cost: u32,
    
    // Parallelism factor
    parallelism: u32,
    
    // Output length in bytes
    output_length: usize,
    
    // Salt (if provided)
    salt: Option<String>,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            // Parameters designed for interactive use (~1 second on modern hardware)
            memory_cost: 64 * 1024, // 64 MB
            time_cost: 3,
            parallelism: 4,
            output_length: 32, // For AES-256
            salt: None,
        }
    }
}

// Encrypted data container
#[derive(Debug, Clone)]
struct EncryptedData {
    // Encrypted content
    ciphertext: Vec<u8>,
    
    // Nonce used for encryption
    nonce: Vec<u8>,
    
    // Salt used for key derivation
    salt: String,
    
    // Key derivation parameters
    kdf_params: KdfParams,
}

// Password-based encryption utilities
struct PasswordEncryption;

impl PasswordEncryption {
    // Derive a key from a password using Argon2id
    pub fn derive_key(
        password: &SecretString,
        params: &KdfParams,
    ) -> EncryptionResult<DerivedKey> {
        // Create Argon2id context
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(
                params.memory_cost,
                params.time_cost,
                params.parallelism,
                Some(params.output_length),
            )
            .map_err(|e| EncryptionError::PasswordHash(e.to_string()))?,
        );
        
        // Generate salt or use provided one
        let salt = match &params.salt {
            Some(salt_str) => SaltString::from_b64(salt_str)
                .map_err(|e| EncryptionError::PasswordHash(format!("Invalid salt: {}", e)))?,
            None => SaltString::generate(&mut OsRng),
        };
        
        // Hash the password
        let password_hash = argon2
            .hash_password(password.expose_secret().as_bytes(), &salt)
            .map_err(|e| EncryptionError::PasswordHash(e.to_string()))?;
        
        // Extract bytes from hash
        let hash_bytes = password_hash
            .hash
            .ok_or_else(|| EncryptionError::PasswordHash("No hash output".to_string()))?
            .as_bytes()
            .to_vec();
        
        Ok(DerivedKey {
            key: Secret::new(hash_bytes),
        })
    }
    
    // Encrypt data with a password
    pub fn encrypt_with_password(
        data: &[u8],
        password: &SecretString,
        params: &KdfParams,
    ) -> EncryptionResult<EncryptedData> {
        // Generate fresh salt
        let salt = SaltString::generate(&mut OsRng);
        let salt_str = salt.as_str().to_string();
        
        // Clone params with the generated salt
        let mut derived_params = params.clone();
        derived_params.salt = Some(salt_str.clone());
        
        // Derive encryption key
        let derived_key = Self::derive_key(password, &derived_params)?;
        
        // Use key for AES-GCM encryption
        let aes_key = Key::<Aes256Gcm>::from_slice(derived_key.expose_key());
        let cipher = Aes256Gcm::new(aes_key);
        
        // Generate random nonce
        let nonce_val = Aes256Gcm::generate_nonce(&mut AesOsRng);
        let nonce_bytes = nonce_val.to_vec();
        
        // Encrypt data
        let ciphertext = cipher
            .encrypt(&nonce_val, data)
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;
        
        Ok(EncryptedData {
            ciphertext,
            nonce: nonce_bytes,
            salt: salt_str,
            kdf_params: derived_params,
        })
    }
    
    // Decrypt data with a password
    pub fn decrypt_with_password(
        encrypted: &EncryptedData,
        password: &SecretString,
    ) -> EncryptionResult<Vec<u8>> {
        // Derive key using the same parameters and salt
        let derived_key = Self::derive_key(password, &encrypted.kdf_params)?;
        
        // Use key for AES-GCM decryption
        let aes_key = Key::<Aes256Gcm>::from_slice(derived_key.expose_key());
        let cipher = Aes256Gcm::new(aes_key);
        
        // Create nonce object
        let nonce = Nonce::from_slice(&encrypted.nonce);
        
        // Decrypt data
        let plaintext = cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;
        
        Ok(plaintext)
    }
}

fn main() {
    // Create a password
    let password = SecretString::new("test_password".to_string());
    
    // Create some data to encrypt
    let data = b"This is a secret message for testing Argon2id encryption";
    
    // Use minimal parameters for fast tests
    let params = KdfParams {
        memory_cost: 1024, // 1 MB
        time_cost: 1,
        parallelism: 1,
        output_length: 32,
        salt: None,
    };
    
    // Encrypt data
    let encrypted = PasswordEncryption::encrypt_with_password(data, &password, &params)
        .expect("Encryption should succeed");
    
    println!("Data encrypted successfully!");
    println!("Ciphertext length: {} bytes", encrypted.ciphertext.len());
    println!("Salt: {}", encrypted.salt);
    
    // Decrypt data
    let decrypted = PasswordEncryption::decrypt_with_password(&encrypted, &password)
        .expect("Decryption should succeed");
    
    // Convert decrypted bytes to string
    let decrypted_str = String::from_utf8_lossy(&decrypted);
    
    println!("Decryption successful!");
    println!("Original: {}", String::from_utf8_lossy(data));
    println!("Decrypted: {}", decrypted_str);
    
    // Verify decrypted data matches original
    assert_eq!(data, &decrypted[..]);
    println!("Verification successful: decrypted data matches original!");
    
    // Try with wrong password
    let wrong_password = SecretString::new("wrong_password".to_string());
    let result = PasswordEncryption::decrypt_with_password(&encrypted, &wrong_password);
    
    match result {
        Ok(_) => println!("ERROR: Decryption with wrong password succeeded!"),
        Err(_) => println!("Test passed: Decryption with wrong password correctly failed"),
    }
} 