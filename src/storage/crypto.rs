use crate::storage::{StorageError, StorageResult};
use crypto_box::{SecretKey, PublicKey, SalsaBox};
use rand::rngs::OsRng;
use zeroize::Zeroize;

/// CryptoStorage provides utilities for securely storing sensitive data
pub struct CryptoStorage {
    /// Encryption key
    secret_key: SecretKey,
    
    /// Corresponding public key
    public_key: PublicKey,
}

impl Drop for CryptoStorage {
    fn drop(&mut self) {
        // Manually zeroize the secret key
        let mut bytes = self.secret_key.as_bytes().to_vec();
        bytes.zeroize();
    }
}

impl CryptoStorage {
    /// Create a new CryptoStorage with a randomly generated key
    pub fn new() -> Self {
        let secret_key = SecretKey::generate(&mut OsRng);
        let public_key = PublicKey::from(&secret_key);
        
        Self {
            secret_key,
            public_key,
        }
    }
    
    /// Create a CryptoStorage from an existing secret key
    pub fn from_secret_key(secret_key_bytes: &[u8]) -> StorageResult<Self> {
        if secret_key_bytes.len() != 32 {
            return Err(StorageError::InvalidData("Invalid secret key length".to_string()));
        }
        
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(secret_key_bytes);
        
        let secret_key = match SecretKey::from(key_bytes) {
            secret_key => secret_key,
        };
        
        let public_key = PublicKey::from(&secret_key);
        
        Ok(Self {
            secret_key,
            public_key,
        })
    }
    
    /// Encrypt data using the internal key
    pub fn encrypt(&self, data: &[u8]) -> StorageResult<Vec<u8>> {
        // Generate a random nonce
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = crypto_box::Nonce::from(nonce_bytes);
        
        // Create recipient public key
        let recipient = self.public_key;
        
        // Create encryption box
        let sender_box = SalsaBox::new(&recipient, &self.secret_key);
        
        // Encrypt the data
        let ciphertext = match sender_box.encrypt(&nonce, data) {
            Ok(ciphertext) => ciphertext,
            Err(_) => return Err(StorageError::Other("Encryption failed".to_string())),
        };
        
        // Combine nonce and ciphertext
        let mut result = Vec::with_capacity(nonce.as_ref().len() + ciphertext.len());
        result.extend_from_slice(nonce.as_ref());
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    /// Decrypt data using the internal key
    pub fn decrypt(&self, encrypted_data: &[u8]) -> StorageResult<Vec<u8>> {
        // Split nonce and ciphertext
        if encrypted_data.len() < 24 {
            return Err(StorageError::InvalidData("Invalid encrypted data".to_string()));
        }
        
        let nonce_bytes = &encrypted_data[..24];
        let ciphertext = &encrypted_data[24..];
        
        // Create nonce
        let mut nonce = [0u8; 24];
        nonce.copy_from_slice(nonce_bytes);
        let nonce = crypto_box::Nonce::from(nonce);
        
        // Create recipient public key
        let recipient = self.public_key;
        
        // Create decryption box
        let sender_box = SalsaBox::new(&recipient, &self.secret_key);
        
        // Decrypt the data
        let plaintext = match sender_box.decrypt(&nonce, ciphertext) {
            Ok(plaintext) => plaintext,
            Err(_) => return Err(StorageError::Other("Decryption failed".to_string())),
        };
        
        Ok(plaintext)
    }
    
    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public_key.as_bytes()
    }
    
    /// Get a copy of the secret key bytes (handle with care)
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        *self.secret_key.as_bytes()
    }
} 