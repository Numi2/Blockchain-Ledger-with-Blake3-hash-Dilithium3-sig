use crate::types::{Address, Hash, Transaction};
use crate::wallet::{KeyStore, WalletError, WalletResult};
use secrecy::{Secret, SecretString, ExposeSecret};
use zeroize::Zeroizing;
use ed25519_dalek::{Keypair as Ed25519Keypair, PublicKey as Ed25519PublicKey, SecretKey as Ed25519SecretKey, Signature as Ed25519Signature};
use rand::{rngs::OsRng, RngCore};
use std::fmt;
use serde::{Serialize, Deserialize};

// Import pqcrypto types
use pqcrypto_traits::sign::{PublicKey as PQPublicKey, SecretKey as PQSecretKey, SignedMessage, DetachedSignature};
use pqcrypto_dilithium::dilithium2;
use pqcrypto_dilithium::dilithium3;
use pqcrypto_dilithium::dilithium5;

/// Supported signature schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureScheme {
    /// Ed25519 signatures
    Ed25519,
    
    /// Dilithium2 post-quantum signatures (NIST level 2 security)
    Dilithium2,
    
    /// Dilithium3 post-quantum signatures (NIST level 3 security)
    Dilithium3,
    
    /// Dilithium5 post-quantum signatures (NIST level 5 security)
    Dilithium5,
}

impl fmt::Display for SignatureScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureScheme::Ed25519 => write!(f, "Ed25519"),
            SignatureScheme::Dilithium2 => write!(f, "Dilithium2"),
            SignatureScheme::Dilithium3 => write!(f, "Dilithium3"),
            SignatureScheme::Dilithium5 => write!(f, "Dilithium5"),
        }
    }
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account address
    pub address: Address,
    
    /// Account name
    pub name: String,
    
    /// Signature scheme used by this account
    pub signature_scheme: SignatureScheme,
    
    /// Key ID in the keystore
    pub key_id: String,
    
    /// Account path (for HD wallets)
    pub derivation_path: Option<String>,
    
    /// Account index (for HD wallets)
    pub index: Option<u32>,
    
    /// Creation timestamp
    pub created_at: u64,
}

/// Manager for accounts and account operations
pub struct AccountManager {
    /// Keystore for secure key management
    keystore: KeyStore,
}

impl AccountManager {
    /// Create a new account manager
    pub fn new(keystore: KeyStore) -> Self {
        Self { keystore }
    }
    
    /// Create a new Ed25519 account
    pub fn create_ed25519_account(&mut self, name: String, password: &SecretString) -> WalletResult<AccountInfo> {
        // Generate a new key
        let key_info = self.keystore.generate_key(name.clone(), password)?;
        
        // Create account info
        let account_info = AccountInfo {
            address: key_info.address,
            name,
            signature_scheme: SignatureScheme::Ed25519,
            key_id: key_info.id,
            derivation_path: None,
            index: None,
            created_at: current_timestamp(),
        };
        
        Ok(account_info)
    }
    
    /// Create a new Dilithium account
    pub fn create_dilithium_account(
        &mut self, 
        name: String, 
        password: &SecretString,
        scheme: SignatureScheme,
    ) -> WalletResult<AccountInfo> {
        // Validate scheme
        match scheme {
            SignatureScheme::Dilithium2 | SignatureScheme::Dilithium3 | SignatureScheme::Dilithium5 => {},
            _ => return Err(WalletError::InvalidKey("Invalid signature scheme for Dilithium".to_string())),
        }
        
        // Generate keypair using pqcrypto
        let (keypair, public_key) = match scheme {
            SignatureScheme::Dilithium2 => {
                let (pk, sk) = dilithium2::keypair();
                (sk, pk)
            },
            SignatureScheme::Dilithium3 => {
                let (pk, sk) = dilithium3::keypair();
                (sk, pk)
            },
            SignatureScheme::Dilithium5 => {
                let (pk, sk) = dilithium5::keypair();
                (sk, pk)
            },
            _ => unreachable!(),
        };
        
        // Get secret key bytes for storage
        let private_key_bytes = keypair.as_bytes().to_vec();
        
        // Store private key in keystore
        let key_info = self.keystore.import_key(&private_key_bytes, name.clone(), password)?;
        
        // Create account info
        let account_info = AccountInfo {
            address: key_info.address,
            name,
            signature_scheme: scheme,
            key_id: key_info.id,
            derivation_path: None,
            index: None,
            created_at: current_timestamp(),
        };
        
        Ok(account_info)
    }
    
    /// Import an existing Ed25519 key
    pub fn import_ed25519_key(
        &mut self,
        private_key: &[u8],
        name: String,
        password: &SecretString,
    ) -> WalletResult<AccountInfo> {
        // Validate key
        if private_key.len() != 32 {
            return Err(WalletError::InvalidKey("Invalid Ed25519 private key length".to_string()));
        }
        
        // Import key to keystore
        let key_info = self.keystore.import_key(private_key, name.clone(), password)?;
        
        // Create account info
        let account_info = AccountInfo {
            address: key_info.address,
            name,
            signature_scheme: SignatureScheme::Ed25519,
            key_id: key_info.id,
            derivation_path: None,
            index: None,
            created_at: current_timestamp(),
        };
        
        Ok(account_info)
    }
    
    /// Import an existing Dilithium key
    pub fn import_dilithium_key(
        &mut self,
        private_key: &[u8],
        name: String,
        password: &SecretString,
        scheme: SignatureScheme,
    ) -> WalletResult<AccountInfo> {
        // Validate scheme
        match scheme {
            SignatureScheme::Dilithium2 | SignatureScheme::Dilithium3 | SignatureScheme::Dilithium5 => {},
            _ => return Err(WalletError::InvalidKey("Invalid signature scheme for Dilithium".to_string())),
        }
        
        // Validate private key (attempt to deserialize it)
        let _ = match scheme {
            SignatureScheme::Dilithium2 => {
                dilithium2::SecretKey::from_bytes(private_key)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium2 private key".to_string()))?
            },
            SignatureScheme::Dilithium3 => {
                dilithium3::SecretKey::from_bytes(private_key)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium3 private key".to_string()))?
            },
            SignatureScheme::Dilithium5 => {
                dilithium5::SecretKey::from_bytes(private_key)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium5 private key".to_string()))?
            },
            _ => unreachable!(),
        };
        
        // Import key to keystore
        let key_info = self.keystore.import_key(private_key, name.clone(), password)?;
        
        // Create account info
        let account_info = AccountInfo {
            address: key_info.address,
            name,
            signature_scheme: scheme,
            key_id: key_info.id,
            derivation_path: None,
            index: None,
            created_at: current_timestamp(),
        };
        
        Ok(account_info)
    }
    
    /// Sign a transaction or message with an account
    pub fn sign_transaction(
        &self,
        account_info: &AccountInfo,
        transaction: &Transaction,
        password: &SecretString,
    ) -> WalletResult<Vec<u8>> {
        match account_info.signature_scheme {
            SignatureScheme::Ed25519 => self.sign_ed25519(account_info, transaction, password),
            SignatureScheme::Dilithium2 => self.sign_dilithium2(account_info, transaction, password),
            SignatureScheme::Dilithium3 => self.sign_dilithium3(account_info, transaction, password),
            SignatureScheme::Dilithium5 => self.sign_dilithium5(account_info, transaction, password),
        }
    }
    
    /// Sign a message using Ed25519
    fn sign_ed25519(
        &self,
        account_info: &AccountInfo,
        transaction: &Transaction,
        password: &SecretString,
    ) -> WalletResult<Vec<u8>> {
        // Get private key from keystore
        let private_key_bytes = self.keystore.get_private_key(&account_info.key_id, password)?;
        
        // Create keypair
        let secret_key = Ed25519SecretKey::from_bytes(&private_key_bytes)
            .map_err(|e| WalletError::InvalidKey(e.to_string()))?;
        
        let public_key_info = self.keystore.get_key_info(&account_info.key_id)
            .ok_or_else(|| WalletError::AccountNotFound(account_info.key_id.clone()))?;
        
        let public_key = Ed25519PublicKey::from_bytes(&public_key_info.public_key)
            .map_err(|e| WalletError::InvalidKey(e.to_string()))?;
        
        let keypair = Ed25519Keypair {
            secret: secret_key,
            public: public_key,
        };
        
        // Serialize transaction for signing
        let message = transaction.hash();
        
        // Sign the message
        let signature = keypair.sign(&message);
        
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Sign a message using Dilithium2
    fn sign_dilithium2(
        &self,
        account_info: &AccountInfo,
        transaction: &Transaction,
        password: &SecretString,
    ) -> WalletResult<Vec<u8>> {
        // Get private key from keystore
        let private_key_bytes = self.keystore.get_private_key(&account_info.key_id, password)?;
        
        // Create secret key from bytes
        let secret_key = dilithium2::SecretKey::from_bytes(&private_key_bytes)
            .map_err(|_| WalletError::InvalidKey("Invalid Dilithium2 private key".to_string()))?;
        
        // Serialize transaction for signing
        let message = transaction.hash();
        
        // Sign the message (detached signature is smaller than signed message)
        let signature = dilithium2::detached_sign(&message, &secret_key);
        
        Ok(signature.as_bytes().to_vec())
    }
    
    /// Sign a message using Dilithium3
    fn sign_dilithium3(
        &self,
        account_info: &AccountInfo,
        transaction: &Transaction,
        password: &SecretString,
    ) -> WalletResult<Vec<u8>> {
        // Get private key from keystore
        let private_key_bytes = self.keystore.get_private_key(&account_info.key_id, password)?;
        
        // Create secret key from bytes
        let secret_key = dilithium3::SecretKey::from_bytes(&private_key_bytes)
            .map_err(|_| WalletError::InvalidKey("Invalid Dilithium3 private key".to_string()))?;
        
        // Serialize transaction for signing
        let message = transaction.hash();
        
        // Sign the message (detached signature is smaller than signed message)
        let signature = dilithium3::detached_sign(&message, &secret_key);
        
        Ok(signature.as_bytes().to_vec())
    }
    
    /// Sign a message using Dilithium5
    fn sign_dilithium5(
        &self,
        account_info: &AccountInfo,
        transaction: &Transaction,
        password: &SecretString,
    ) -> WalletResult<Vec<u8>> {
        // Get private key from keystore
        let private_key_bytes = self.keystore.get_private_key(&account_info.key_id, password)?;
        
        // Create secret key from bytes
        let secret_key = dilithium5::SecretKey::from_bytes(&private_key_bytes)
            .map_err(|_| WalletError::InvalidKey("Invalid Dilithium5 private key".to_string()))?;
        
        // Serialize transaction for signing
        let message = transaction.hash();
        
        // Sign the message (detached signature is smaller than signed message)
        let signature = dilithium5::detached_sign(&message, &secret_key);
        
        Ok(signature.as_bytes().to_vec())
    }
    
    /// Verify a signature
    pub fn verify_signature(
        &self,
        signature_scheme: SignatureScheme,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> WalletResult<bool> {
        match signature_scheme {
            SignatureScheme::Ed25519 => {
                // Parse public key
                let public_key = Ed25519PublicKey::from_bytes(public_key)
                    .map_err(|e| WalletError::InvalidKey(e.to_string()))?;
                
                // Parse signature
                let signature = Ed25519Signature::from_bytes(signature)
                    .map_err(|e| WalletError::InvalidKey(e.to_string()))?;
                
                // Verify
                Ok(public_key.verify(message, &signature).is_ok())
            },
            SignatureScheme::Dilithium2 => {
                // Parse public key
                let public_key = dilithium2::PublicKey::from_bytes(public_key)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium2 public key".to_string()))?;
                
                // Parse signature
                let detached_signature = dilithium2::DetachedSignature::from_bytes(signature)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium2 signature".to_string()))?;
                
                // Verify - returns Result<(), _> where Ok means verification succeeded
                let result = dilithium2::verify_detached_signature(&detached_signature, message, &public_key);
                Ok(result.is_ok())
            },
            SignatureScheme::Dilithium3 => {
                // Parse public key
                let public_key = dilithium3::PublicKey::from_bytes(public_key)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium3 public key".to_string()))?;
                
                // Parse signature
                let detached_signature = dilithium3::DetachedSignature::from_bytes(signature)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium3 signature".to_string()))?;
                
                // Verify
                let result = dilithium3::verify_detached_signature(&detached_signature, message, &public_key);
                Ok(result.is_ok())
            },
            SignatureScheme::Dilithium5 => {
                // Parse public key
                let public_key = dilithium5::PublicKey::from_bytes(public_key)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium5 public key".to_string()))?;
                
                // Parse signature
                let detached_signature = dilithium5::DetachedSignature::from_bytes(signature)
                    .map_err(|_| WalletError::InvalidKey("Invalid Dilithium5 signature".to_string()))?;
                
                // Verify
                let result = dilithium5::verify_detached_signature(&detached_signature, message, &public_key);
                Ok(result.is_ok())
            },
        }
    }
}

/// Generate a random seed for key generation
fn generate_random_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    seed
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_ed25519_account_creation() {
        let temp_dir = tempdir().unwrap();
        let keystore = KeyStore::new(temp_dir.path().to_path_buf()).unwrap();
        let mut account_manager = AccountManager::new(keystore);
        
        let password = SecretString::new("test_password".to_string());
        let account = account_manager.create_ed25519_account("Test Account".to_string(), &password).unwrap();
        
        assert_eq!(account.name, "Test Account");
        assert_eq!(account.signature_scheme, SignatureScheme::Ed25519);
    }
    
    // Additional tests would be added here for Dilithium
} 