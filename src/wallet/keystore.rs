// src/wallet/keystore.rs
use crate::storage::{Database, Column, StorageError};
use crate::wallet::{WalletResult, WalletError};
use crate::types::Address;
use secrecy::{Secret, SecretString, ExposeSecret};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use crypto_box::{SecretKey, PublicKey, SalsaBox};
use crypto_box::aead::{Aead, Nonce};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::{Zeroize, Zeroizing};
use blake3;
use crate::wallet::encryption::{PasswordEncryption, KdfParams, EncryptedData};
use thiserror::Error;
use uuid::Uuid;
use tracing::{debug, error, info, warn};
use aes_gcm::{Aes256Gcm, Key};

/// KeyStore error
#[derive(Error, Debug)]
pub enum KeyStoreError {
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Account locked: {0}")]
    AccountLocked(String),
}

/// Result type for KeyStore operations
pub type KeyStoreResult<T> = Result<T, KeyStoreError>;

/// EncryptedData represents encrypted data with its nonce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted data
    pub ciphertext: Vec<u8>,
    
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
}

/// KeyInfo stores information about a key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// Key ID (UUID)
    pub id: String,
    
    /// Public key derived from the private key
    pub public_key: Vec<u8>,
    
    /// Address derived from this key
    pub address: Address,
    
    /// User-friendly name for this key
    pub name: String,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Last rotation timestamp
    pub rotated_at: Option<u64>,
    
    /// Failed password attempts
    pub failed_attempts: u32,
    
    /// Locked until timestamp
    pub locked_until: Option<u64>,
}

/// Encrypted key file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeyFile {
    /// Key information
    pub info: KeyInfo,
    
    /// Key encryption method
    pub encryption: String,
    
    /// Encrypted private key data
    pub encrypted_data: EncryptedData,
    
    /// Key derivation parameters (for password-based encryption)
    pub kdf_params: Option<KdfParams>,
    
    /// Key version
    pub version: u32,
}

/// Key rotation status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRotationStatus {
    /// Key rotation not needed
    NotNeeded,
    
    /// Key rotation recommended (but not urgent)
    Recommended,
    
    /// Key rotation required as soon as possible
    Required,
}

/// KeyStore manages keys and performs cryptographic operations
pub struct KeyStore {
    /// Database for storing keys
    db: Option<Database>,
    
    /// Directory for key files
    keys_dir: PathBuf,
    
    /// In-memory cache of key info
    key_cache: HashMap<String, KeyInfo>,
    
    /// Key derivation parameters
    kdf_params: KdfParams,
    
    /// Maximum failed password attempts
    max_password_attempts: u32,
    
    /// Lockout period in seconds
    lockout_secs: u64,
    
    /// Key rotation period in seconds (0 = never)
    rotation_secs: u64,
}

impl KeyStore {
    /// Create a new keystore with a database
    pub fn new_with_db(db: Database, keys_dir: PathBuf) -> WalletResult<Self> {
        // Create the keys directory if it doesn't exist
        if !keys_dir.exists() {
            fs::create_dir_all(&keys_dir)?;
        }
        
        // Scan existing keys
        let mut keystore = Self {
            db: Some(db),
            keys_dir: keys_dir.clone(),
            key_cache: HashMap::new(),
            kdf_params: KdfParams::default(),
            max_password_attempts: 5,
            lockout_secs: 15 * 60, // 15 minutes
            rotation_secs: 90 * 24 * 60 * 60, // 90 days
        };
        
        // Load key info from files
        keystore.load_key_info()?;
        
        Ok(keystore)
    }
    
    /// Create a new file-based keystore without a database
    pub fn new(keys_dir: PathBuf) -> WalletResult<Self> {
        // Create the keys directory if it doesn't exist
        if !keys_dir.exists() {
            fs::create_dir_all(&keys_dir)?;
        }
        
        // Scan existing keys
        let mut keystore = Self {
            db: None,
            keys_dir: keys_dir.clone(),
            key_cache: HashMap::new(),
            kdf_params: KdfParams::default(),
            max_password_attempts: 5,
            lockout_secs: 15 * 60, // 15 minutes
            rotation_secs: 90 * 24 * 60 * 60, // 90 days
        };
        
        // Load key info from files
        keystore.load_key_info()?;
        
        Ok(keystore)
    }
    
    /// Load key info from the keystore directory
    fn load_key_info(&mut self) -> WalletResult<()> {
        // Read files from the keystore directory
        let entries = fs::read_dir(&self.keys_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Only process JSON files
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(key_file) = self.read_key_file(&path) {
                    // Add to cache
                    self.key_cache.insert(key_file.info.id.clone(), key_file.info);
                }
            }
        }
        
        Ok(())
    }
    
    /// Read a key file from disk
    fn read_key_file<P: AsRef<Path>>(&self, path: P) -> WalletResult<EncryptedKeyFile> {
        // Open the file
        let mut file = File::open(path)?;
        
        // Read the contents
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        // Deserialize
        let key_file: EncryptedKeyFile = serde_json::from_str(&contents)
            .map_err(|e| WalletError::Other(format!("Failed to deserialize key file: {}", e)))?;
        
        Ok(key_file)
    }
    
    /// Write a key file to disk
    fn write_key_file(&self, key_file: &EncryptedKeyFile) -> WalletResult<PathBuf> {
        // Create file path
        let file_name = format!("key-{}.json", key_file.info.id);
        let path = self.keys_dir.join(file_name);
        
        // Serialize
        let contents = serde_json::to_string_pretty(key_file)
            .map_err(|e| WalletError::Other(format!("Failed to serialize key file: {}", e)))?;
        
        // Write to file
        let mut file = File::create(&path)?;
        file.write_all(contents.as_bytes())?;
        
        Ok(path)
    }
    
    /// Generate a new key
    pub fn generate_key(&mut self, name: String, password: &SecretString) -> WalletResult<KeyInfo> {
        // Generate a new key
        let secret_key = SecretKey::generate(&mut OsRng);
        let public_key = PublicKey::from(&secret_key);
        
        // Generate an address from the public key
        let address = self.derive_address(&public_key.as_bytes().to_vec())?;
        
        // Generate a unique ID
        let id = generate_uuid();
        
        // Create key info
        let key_info = KeyInfo {
            id: id.clone(),
            public_key: public_key.as_bytes().to_vec(),
            address,
            name,
            created_at: current_timestamp(),
            rotated_at: None,
            failed_attempts: 0,
            locked_until: None,
        };
        
        // Encrypt the private key
        let encrypted_key = self.encrypt_key(&secret_key.as_bytes().to_vec(), password)?;
        
        // Create key file
        let key_file = EncryptedKeyFile {
            info: key_info.clone(),
            encryption: "nacl".to_string(),
            encrypted_data: encrypted_key,
            kdf_params: None, // Not using KDF for this implementation
            version: 1,
        };
        
        // Write key file
        self.write_key_file(&key_file)?;
        
        // Add to cache
        self.key_cache.insert(id, key_info.clone());
        
        // If we have a database, store there too
        if let Some(db) = &self.db {
            let key_file_bytes = serde_json::to_vec(&key_file)
                .map_err(|e| WalletError::Other(format!("Failed to serialize key file: {}", e)))?;
                
            db.put(Column::Wallets, key_info.id.as_bytes(), &key_file_bytes)
                .map_err(|e| WalletError::Storage(e))?;
        }
        
        Ok(key_info)
    }
    
    /// Import an existing key
    pub fn import_key(&mut self, private_key: &[u8], name: String, password: &SecretString) -> WalletResult<KeyInfo> {
        // Create secret key from bytes
        if private_key.len() != 32 {
            return Err(WalletError::InvalidKey("Invalid private key length".to_string()));
        }
        
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(private_key);
        
        let secret_key = SecretKey::from(key_bytes);
        let public_key = PublicKey::from(&secret_key);
        
        // Generate an address from the public key
        let address = self.derive_address(&public_key.as_bytes().to_vec())?;
        
        // Generate a unique ID
        let id = generate_uuid();
        
        // Create key info
        let key_info = KeyInfo {
            id: id.clone(),
            public_key: public_key.as_bytes().to_vec(),
            address,
            name,
            created_at: current_timestamp(),
        };
        
        // Encrypt the private key
        let encrypted_key = self.encrypt_key(&secret_key.as_bytes().to_vec(), password)?;
        
        // Create key file
        let key_file = EncryptedKeyFile {
            info: key_info.clone(),
            encryption: "nacl".to_string(),
            encrypted_data: encrypted_key,
            kdf_params: None, // Not using KDF for this implementation
            version: 1,
        };
        
        // Write key file
        self.write_key_file(&key_file)?;
        
        // Add to cache
        self.key_cache.insert(id, key_info.clone());
        
        // If we have a database, store there too
        if let Some(db) = &self.db {
            let key_file_bytes = serde_json::to_vec(&key_file)
                .map_err(|e| WalletError::Other(format!("Failed to serialize key file: {}", e)))?;
                
            db.put(Column::Wallets, key_info.id.as_bytes(), &key_file_bytes)
                .map_err(|e| WalletError::Storage(e))?;
        }
        
        Ok(key_info)
    }
    
    /// Get the private key by ID
    pub fn get_private_key(&self, id: &str, password: &SecretString) -> WalletResult<Zeroizing<Vec<u8>>> {
        // Create file path
        let file_name = format!("key-{}.json", id);
        let path = self.keys_dir.join(file_name);
        
        // Read key file
        let key_file = self.read_key_file(path)?;
        
        // Decrypt the private key
        let private_key = self.decrypt_key(&key_file.encrypted_data, password)?;
        
        Ok(private_key)
    }
    
    /// Get all key info
    pub fn get_all_keys(&self) -> Vec<KeyInfo> {
        self.key_cache.values().cloned().collect()
    }
    
    /// Get key info by ID
    pub fn get_key_info(&self, id: &str) -> Option<KeyInfo> {
        self.key_cache.get(id).cloned()
    }
    
    /// Get key info by address
    pub fn get_key_info_by_address(&self, address: &Address) -> Option<KeyInfo> {
        self.key_cache.values()
            .find(|info| info.address == *address)
            .cloned()
    }
    
    /// Delete a key by ID
    pub fn delete_key(&mut self, id: &str) -> WalletResult<()> {
        // Remove from cache
        self.key_cache.remove(id);
        
        // Delete file
        let file_name = format!("key-{}.json", id);
        let path = self.keys_dir.join(file_name);
        
        if path.exists() {
            fs::remove_file(path)?;
        }
        
        // If we have a database, delete there too
        if let Some(db) = &self.db {
            db.delete(Column::Wallets, id.as_bytes())
                .map_err(|e| WalletError::Storage(e))?;
        }
        
        Ok(())
    }
    
    /// Encrypt a private key
    fn encrypt_key(&self, private_key: &[u8], password: &SecretString) -> WalletResult<EncryptedData> {
        // Step 1: Derive key using Argon2id with proper parameters
        let derived_key = PasswordEncryption::derive_key(
            password, 
            &self.kdf_params
        ).map_err(|e| WalletError::Crypto(format!("Key derivation failed: {}", e)))?;
        
        // Step 2: Apply BLAKE3 for additional quantum resistance
        let mut blake3_context = blake3::Hasher::new();
        blake3_context.update(derived_key.expose_key());
        let hardened_key = blake3_context.finalize();
        
        // Step 3: Use modern AES-GCM for authenticated encryption
        let aes_key = Key::<Aes256Gcm>::from_slice(hardened_key.as_bytes());
        let cipher = Aes256Gcm::new(aes_key);
        
        // Step 4: Generate random nonce
        let nonce_val = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce_bytes = nonce_val.to_vec();
        
        // Step 5: Encrypt with authentication tag
        let ciphertext = cipher
            .encrypt(&nonce_val, private_key)
            .map_err(|e| WalletError::Crypto(format!("Encryption failed: {}", e)))?;
        
        Ok(EncryptedData {
            ciphertext,
            nonce: nonce_bytes,
        })
    }
    
    /// Decrypt a private key
    fn decrypt_key(&self, encrypted_data: &EncryptedData, password: &SecretString) -> WalletResult<Zeroizing<Vec<u8>>> {
        // Step 1: Derive key using Argon2id with proper parameters (same as encrypt)
        let derived_key = PasswordEncryption::derive_key(
            password, 
            &self.kdf_params
        ).map_err(|e| WalletError::Crypto(format!("Key derivation failed: {}", e)))?;
        
        // Step 2: Apply BLAKE3 for additional quantum resistance (same as encrypt)
        let mut blake3_context = blake3::Hasher::new();
        blake3_context.update(derived_key.expose_key());
        let hardened_key = blake3_context.finalize();
        
        // Step 3: Use AES-GCM for authenticated decryption
        let aes_key = Key::<Aes256Gcm>::from_slice(hardened_key.as_bytes());
        let cipher = Aes256Gcm::new(aes_key);
        
        // Step 4: Create nonce object from stored nonce bytes
        let nonce = aes_gcm::Nonce::from_slice(&encrypted_data.nonce);
        
        // Step 5: Decrypt with authentication verification
        let plaintext = cipher
            .decrypt(nonce, encrypted_data.ciphertext.as_ref())
            .map_err(|_| WalletError::InvalidPassword)?;
        
        // Return with zeroizing wrapper for security
        Ok(Zeroizing::new(plaintext))
    }
    
    /// Derive an address from a public key
    fn derive_address(&self, public_key: &[u8]) -> WalletResult<Address> {
        // Hash the public key with BLAKE3
        let hash = blake3::hash(public_key);
        
        // Take the last 20 bytes as the address
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash.as_bytes()[12..32]);
        
        Ok(address)
    }
}

/// Generate a unique identifier (UUID v4)
fn generate_uuid() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let uuid = uuid::Uuid::from_bytes([
        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
    ]);
    
    uuid.to_string()
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Derive an encryption key from a password
fn derive_encryption_key(password: &[u8]) -> WalletResult<[u8; 32]> {
    // For simplicity, we use BLAKE3 for key derivation
    let hash = blake3::keyed_hash(&[0u8; 32], password);
    
    Ok(*hash.as_bytes())
} 