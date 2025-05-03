use crate::types::{Hash, Address, Signature};
use crate::wallet::{KeyStore, KeyStoreError, KeyRotationStatus};
use crate::crypto::{SignatureScheme, KeyPair};
use secrecy::SecretString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Key rotation error
#[derive(Error, Debug)]
pub enum KeyRotationError {
    #[error("KeyStore error: {0}")]
    KeyStoreError(#[from] KeyStoreError),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Rotation not needed")]
    NotNeeded,
    
    #[error("Invalid signing key")]
    InvalidSigningKey,
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for key rotation operations
pub type KeyRotationResult<T> = Result<T, KeyRotationError>;

/// Rotation target type (what keys need rotation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationTarget {
    /// Validator consensus key
    ConsensusKey,
    
    /// Block signing key
    BlockSigningKey,
    
    /// Withdrawal key
    WithdrawalKey,
    
    /// All keys
    All,
}

/// Key rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    /// Whether automatic rotation is enabled
    pub enabled: bool,
    
    /// Rotation period in days for consensus key
    pub consensus_key_days: u32,
    
    /// Rotation period in days for block signing key
    pub block_signing_key_days: u32,
    
    /// Rotation period in days for withdrawal key
    pub withdrawal_key_days: u32,
    
    /// Warning period in days before rotation is required
    pub warning_days: u32,
    
    /// Whether to use quantum-resistant keys for rotation
    pub use_quantum_resistant: bool,
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            consensus_key_days: 30, // 1 month
            block_signing_key_days: 90, // 3 months
            withdrawal_key_days: 365, // 1 year
            warning_days: 7, // 1 week
            use_quantum_resistant: true,
        }
    }
}

/// Validator key set
pub struct ValidatorKeys {
    /// Key store
    keystore: KeyStore,
    
    /// Consensus key ID
    consensus_key_id: String,
    
    /// Block signing key ID
    block_signing_key_id: String,
    
    /// Withdrawal key ID (optional)
    withdrawal_key_id: Option<String>,
    
    /// Rotation configuration
    config: KeyRotationConfig,
}

impl ValidatorKeys {
    /// Create a new validator key set
    pub fn new(
        keystore: KeyStore,
        consensus_key_id: String,
        block_signing_key_id: String,
        withdrawal_key_id: Option<String>,
        config: KeyRotationConfig,
    ) -> Self {
        Self {
            keystore,
            consensus_key_id,
            block_signing_key_id,
            withdrawal_key_id,
            config,
        }
    }
    
    /// Set the rotation configuration
    pub fn set_config(&mut self, config: KeyRotationConfig) {
        self.config = config;
    }
    
    /// Get the rotation configuration
    pub fn config(&self) -> &KeyRotationConfig {
        &self.config
    }
    
    /// Check rotation status for all keys
    pub fn check_rotation_status(&self) -> HashMap<RotationTarget, KeyRotationStatus> {
        let mut status = HashMap::new();
        
        // Check consensus key
        status.insert(
            RotationTarget::ConsensusKey,
            self.check_key_rotation_status(&self.consensus_key_id, self.config.consensus_key_days),
        );
        
        // Check block signing key
        status.insert(
            RotationTarget::BlockSigningKey,
            self.check_key_rotation_status(&self.block_signing_key_id, self.config.block_signing_key_days),
        );
        
        // Check withdrawal key if present
        if let Some(ref key_id) = self.withdrawal_key_id {
            status.insert(
                RotationTarget::WithdrawalKey,
                self.check_key_rotation_status(key_id, self.config.withdrawal_key_days),
            );
        }
        
        status
    }
    
    /// Check rotation status for a specific key
    fn check_key_rotation_status(&self, key_id: &str, rotation_days: u32) -> KeyRotationStatus {
        // If rotation is disabled, no rotation needed
        if !self.config.enabled {
            return KeyRotationStatus::NotNeeded;
        }
        
        // Get key info
        let key_info = match self.keystore.get_key(key_id) {
            Ok(info) => info,
            Err(_) => return KeyRotationStatus::NotNeeded, // Key not found
        };
        
        // Get last rotation time (or creation time if never rotated)
        let last_rotation = key_info.rotated_at.unwrap_or(key_info.created_at);
        
        // Calculate seconds in rotation period
        let rotation_secs = rotation_days as u64 * 24 * 60 * 60;
        
        // Calculate time since last rotation
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let time_since_rotation = now.saturating_sub(last_rotation);
        
        // Calculate warning threshold
        let warning_secs = self.config.warning_days as u64 * 24 * 60 * 60;
        let warning_threshold = rotation_secs.saturating_sub(warning_secs);
        
        // Check rotation status based on thresholds
        if time_since_rotation >= rotation_secs {
            KeyRotationStatus::Required
        } else if time_since_rotation >= warning_threshold {
            KeyRotationStatus::Recommended
        } else {
            KeyRotationStatus::NotNeeded
        }
    }
    
    /// Rotate a specific key
    pub fn rotate_key(
        &mut self,
        target: RotationTarget,
        password: &SecretString,
    ) -> KeyRotationResult<String> {
        // Get key ID based on target
        let key_id = match target {
            RotationTarget::ConsensusKey => self.consensus_key_id.clone(),
            RotationTarget::BlockSigningKey => self.block_signing_key_id.clone(),
            RotationTarget::WithdrawalKey => {
                self.withdrawal_key_id.clone()
                    .ok_or(KeyRotationError::Other("No withdrawal key configured".to_string()))?
            },
            RotationTarget::All => {
                // Rotate all keys
                let consensus_result = self.rotate_key(RotationTarget::ConsensusKey, password)?;
                let block_result = self.rotate_key(RotationTarget::BlockSigningKey, password)?;
                
                let withdrawal_result = if self.withdrawal_key_id.is_some() {
                    match self.rotate_key(RotationTarget::WithdrawalKey, password) {
                        Ok(id) => format!(", withdrawal={}", id),
                        Err(_) => "".to_string(),
                    }
                } else {
                    "".to_string()
                };
                
                return Ok(format!("consensus={}, block={}{}", consensus_result, block_result, withdrawal_result));
            }
        };
        
        // Check if rotation is needed
        let rotation_days = match target {
            RotationTarget::ConsensusKey => self.config.consensus_key_days,
            RotationTarget::BlockSigningKey => self.config.block_signing_key_days,
            RotationTarget::WithdrawalKey => self.config.withdrawal_key_days,
            RotationTarget::All => unreachable!(),
        };
        
        let status = self.check_key_rotation_status(&key_id, rotation_days);
        if status == KeyRotationStatus::NotNeeded && !self.config.enabled {
            return Err(KeyRotationError::NotNeeded);
        }
        
        // Get key info
        let key_info = self.keystore.get_key(&key_id)?;
        
        // Decrypt the private key
        let private_key = self.keystore.decrypt_key(&key_id, password)?;
        
        // Generate a new key pair
        let key_type = if self.config.use_quantum_resistant {
            "dilithium3" // Use quantum-resistant key
        } else {
            match key_info.key_type.as_str() {
                "ed25519" => "ed25519",
                "dilithium2" | "dilithium3" | "dilithium5" => key_info.key_type.as_str(),
                _ => "dilithium3", // Default to quantum-resistant
            }
        };
        
        // Create a new key pair
        let (new_private_key, new_public_key, new_address) = self.generate_key_pair(key_type)?;
        
        // Store the new key
        let new_key_id = format!("{}_rotated_{}", key_info.name, Self::current_timestamp());
        let new_key_info = self.keystore.create_key(
            &new_key_id,
            key_type,
            &new_private_key,
            &new_public_key,
            &new_address,
            password,
        )?;
        
        // Update internal key ID
        match target {
            RotationTarget::ConsensusKey => self.consensus_key_id = new_key_info.id.clone(),
            RotationTarget::BlockSigningKey => self.block_signing_key_id = new_key_info.id.clone(),
            RotationTarget::WithdrawalKey => {
                if let Some(withdrawal_id) = self.withdrawal_key_id.as_mut() {
                    *withdrawal_id = new_key_info.id.clone();
                }
            },
            RotationTarget::All => unreachable!(),
        }
        
        // Return the new key ID
        Ok(new_key_info.id)
    }
    
    /// Generate a new key pair
    fn generate_key_pair(&self, key_type: &str) -> KeyRotationResult<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        // Create key pair based on type
        match key_type {
            "ed25519" => {
                let keypair = KeyPair::generate_ed25519()
                    .map_err(|e| KeyRotationError::Other(format!("Failed to generate Ed25519 key: {}", e)))?;
                    
                Ok((
                    keypair.private_key().to_vec(),
                    keypair.public_key().to_vec(),
                    keypair.address().to_vec(),
                ))
            },
            "dilithium3" => {
                let keypair = KeyPair::generate_dilithium(SignatureScheme::Dilithium3)
                    .map_err(|e| KeyRotationError::Other(format!("Failed to generate Dilithium key: {}", e)))?;
                    
                Ok((
                    keypair.private_key().to_vec(),
                    keypair.public_key().to_vec(),
                    keypair.address().to_vec(),
                ))
            },
            _ => Err(KeyRotationError::Other(format!("Unsupported key type: {}", key_type))),
        }
    }
    
    /// Get the current timestamp in seconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::wallet::encryption::KdfParams;
    
    #[test]
    fn test_key_rotation() {
        // Create temporary directory for test
        let dir = tempdir().unwrap();
        let mut keystore = KeyStore::new(dir.path()).unwrap();
        
        // Minimal KDF parameters for testing
        keystore.set_kdf_params(KdfParams {
            memory_cost: 1024, // 1 MB
            time_cost: 1,
            parallelism: 1,
            output_length: 32,
            salt: None,
        });
        
        // Create test keys
        let password = SecretString::new("test_password".to_string());
        
        // Consensus key
        let consensus_key = keystore.create_key(
            "consensus_key",
            "dilithium3",
            b"consensus_private_key",
            b"consensus_public_key",
            b"consensus_address",
            &password,
        ).unwrap();
        
        // Block signing key
        let block_key = keystore.create_key(
            "block_key",
            "ed25519",
            b"block_private_key",
            b"block_public_key",
            b"block_address",
            &password,
        ).unwrap();
        
        // Create validator keys
        let config = KeyRotationConfig {
            enabled: true,
            consensus_key_days: 30,
            block_signing_key_days: 90,
            withdrawal_key_days: 365,
            warning_days: 7,
            use_quantum_resistant: true,
        };
        
        let mut validator_keys = ValidatorKeys::new(
            keystore,
            consensus_key.id,
            block_key.id,
            None, // No withdrawal key
            config,
        );
        
        // Check initial rotation status
        let status = validator_keys.check_rotation_status();
        assert_eq!(status[&RotationTarget::ConsensusKey], KeyRotationStatus::NotNeeded);
        assert_eq!(status[&RotationTarget::BlockSigningKey], KeyRotationStatus::NotNeeded);
        
        // Manually force rotation status for testing
        let mut test_config = validator_keys.config().clone();
        test_config.consensus_key_days = 0; // Force immediate rotation
        validator_keys.set_config(test_config);
        
        // Check rotation status after config change
        let status = validator_keys.check_rotation_status();
        assert_eq!(status[&RotationTarget::ConsensusKey], KeyRotationStatus::Required);
        
        // Rotate the consensus key
        let new_key_id = validator_keys.rotate_key(RotationTarget::ConsensusKey, &password).unwrap();
        
        // Verify the key was rotated
        assert_ne!(new_key_id, consensus_key.id);
        assert_eq!(validator_keys.consensus_key_id, new_key_id);
        
        // Check rotation status after rotation
        let status = validator_keys.check_rotation_status();
        assert_eq!(status[&RotationTarget::ConsensusKey], KeyRotationStatus::NotNeeded);
    }
} 