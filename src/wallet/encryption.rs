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
use thiserror::Error;
use std::fmt;

/// Encryption error
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Password hash error: {0}")]
    PasswordHash(String),
    
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("Decryption error: {0}")]
    Decryption(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Encryption key derived from password
#[derive(Clone, ZeroizeOnDrop)]
pub struct DerivedKey {
    /// The key bytes
    #[zeroize(skip)]
    key: Secret<Vec<u8>>,
}

impl fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl DerivedKey {
    /// Get the key bytes
    pub fn expose_key(&self) -> &[u8] {
        self.key.expose_secret()
    }
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory cost (in KB)
    pub memory_cost: u32,
    
    /// Time cost (iterations)
    pub time_cost: u32,
    
    /// Parallelism factor
    pub parallelism: u32,
    
    /// Output length in bytes
    pub output_length: usize,
    
    /// Salt (if provided)
    pub salt: Option<String>,
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

/// Security sensitive parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityParams {
    /// Key derivation parameters
    pub kdf: KdfParams,
    
    /// Key rotation interval in days (0 = never)
    pub key_rotation_days: u32,
    
    /// Maximum failed password attempts before lockout
    pub max_password_attempts: u32,
    
    /// Lockout period in minutes after max attempts
    pub lockout_minutes: u32,
}

impl Default for SecurityParams {
    fn default() -> Self {
        Self {
            kdf: KdfParams::default(),
            key_rotation_days: 90, // 3 months
            max_password_attempts: 5,
            lockout_minutes: 15,
        }
    }
}

/// Encrypted data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted content
    pub ciphertext: Vec<u8>,
    
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    
    /// Salt used for key derivation
    pub salt: String,
    
    /// Key derivation parameters
    pub kdf_params: KdfParams,
}

/// Password-based encryption utilities
pub struct PasswordEncryption;

impl PasswordEncryption {
    /// Derive a key from a password using Argon2id
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
    
    /// Verify a password against a previously derived key
    pub fn verify_password(
        password: &SecretString,
        params: &KdfParams,
        salt: &str,
        expected_key: &[u8],
    ) -> EncryptionResult<bool> {
        // Create Argon2id context with the same parameters
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
        
        // Parse the stored salt
        let salt = SaltString::from_b64(salt)
            .map_err(|e| EncryptionError::PasswordHash(format!("Invalid salt: {}", e)))?;
        
        // Hash the password with the same salt
        let password_hash = argon2
            .hash_password(password.expose_secret().as_bytes(), &salt)
            .map_err(|e| EncryptionError::PasswordHash(e.to_string()))?;
        
        // Extract bytes from hash
        let hash_bytes = password_hash
            .hash
            .ok_or_else(|| EncryptionError::PasswordHash("No hash output".to_string()))?
            .as_bytes();
        
        // Constant-time comparison
        Ok(constant_time_eq(hash_bytes, expected_key))
    }
    
    /// Encrypt data with a password
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
    
    /// Decrypt data with a password
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
    
    /// Update the KDF parameters of encrypted data (requires re-encryption)
    pub fn update_kdf_params(
        encrypted: &EncryptedData,
        password: &SecretString,
        new_params: &KdfParams,
    ) -> EncryptionResult<EncryptedData> {
        // First decrypt the data
        let plaintext = Self::decrypt_with_password(encrypted, password)?;
        
        // Then re-encrypt with new parameters
        Self::encrypt_with_password(&plaintext, password, new_params)
    }
}

/// Constant-time comparison of two byte slices
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt() {
        let password = SecretString::new("test_password".to_string());
        let data = b"secret data to encrypt";
        
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
        
        // Verify ciphertext is not the same as plaintext
        assert_ne!(&encrypted.ciphertext, data);
        
        // Decrypt data
        let decrypted = PasswordEncryption::decrypt_with_password(&encrypted, &password)
            .expect("Decryption should succeed");
        
        // Verify decrypted matches original
        assert_eq!(&decrypted, data);
        
        // Try with wrong password
        let wrong_password = SecretString::new("wrong_password".to_string());
        let result = PasswordEncryption::decrypt_with_password(&encrypted, &wrong_password);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_update_kdf_params() {
        let password = SecretString::new("test_password".to_string());
        let data = b"secret data to encrypt";
        
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
        
        // Update parameters
        let new_params = KdfParams {
            memory_cost: 2048, // 2 MB
            time_cost: 2,
            parallelism: 2,
            output_length: 32,
            salt: None,
        };
        
        let updated = PasswordEncryption::update_kdf_params(&encrypted, &password, &new_params)
            .expect("Parameter update should succeed");
        
        // Verify parameters were updated
        assert_eq!(updated.kdf_params.memory_cost, 2048);
        assert_eq!(updated.kdf_params.time_cost, 2);
        
        // Decrypt with updated parameters
        let decrypted = PasswordEncryption::decrypt_with_password(&updated, &password)
            .expect("Decryption should succeed");
        
        // Verify decrypted matches original
        assert_eq!(&decrypted, data);
    }
    
    #[test]
    fn test_verify_password() {
        let password = SecretString::new("test_password".to_string());
        
        // Use minimal parameters for fast tests
        let params = KdfParams {
            memory_cost: 1024, // 1 MB
            time_cost: 1,
            parallelism: 1,
            output_length: 32,
            salt: None,
        };
        
        // Derive key
        let derived = PasswordEncryption::derive_key(&password, &params)
            .expect("Key derivation should succeed");
        
        // Verify correct password
        let result = PasswordEncryption::verify_password(
            &password,
            &params,
            params.salt.as_ref().unwrap(),
            derived.expose_key(),
        ).expect("Verification should succeed");
        
        assert!(result);
        
        // Verify incorrect password
        let wrong_password = SecretString::new("wrong_password".to_string());
        let result = PasswordEncryption::verify_password(
            &wrong_password,
            &params,
            params.salt.as_ref().unwrap(),
            derived.expose_key(),
        ).expect("Verification should run");
        
        assert!(!result);
    }
} 