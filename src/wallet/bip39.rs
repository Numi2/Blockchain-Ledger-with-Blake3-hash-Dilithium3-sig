use crate::wallet::WalletResult;
use bip39::{Language, Mnemonic};
use slip10::{derive_key_from_path, BIP32Path, Curve};
use zeroize::{Zeroize, Zeroizing};
use std::str::FromStr;
use secrecy::{Secret, SecretString};

/// Default derivation path for World Ledger accounts
pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/6060'/0'/0/0";

/// Strength of the mnemonic (number of bits of entropy)
pub enum MnemonicStrength {
    /// 128 bits = 12 words
    Low = 128,
    /// 160 bits = 15 words
    Medium = 160,
    /// 192 bits = 18 words
    High = 192,
    /// 224 bits = 21 words
    VeryHigh = 224,
    /// 256 bits = 24 words
    Maximum = 256,
}

/// MnemonicBuilder helps with creating and validating mnemonics
pub struct MnemonicBuilder {
    strength: MnemonicStrength,
    language: Language,
    passphrase: Option<SecretString>,
}

impl Default for MnemonicBuilder {
    fn default() -> Self {
        Self {
            strength: MnemonicStrength::Medium,
            language: Language::English,
            passphrase: None,
        }
    }
}

impl MnemonicBuilder {
    /// Create a new MnemonicBuilder with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the strength of the mnemonic
    pub fn strength(mut self, strength: MnemonicStrength) -> Self {
        self.strength = strength;
        self
    }
    
    /// Set the language of the mnemonic
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    
    /// Set an optional passphrase for additional security
    pub fn passphrase<S: AsRef<str>>(mut self, passphrase: S) -> Self {
        self.passphrase = Some(SecretString::new(passphrase.as_ref().to_string()));
        self
    }
    
    /// Generate a new random mnemonic
    pub fn generate(&self) -> WalletResult<Mnemonic> {
        let mnemonic = Mnemonic::new(self.strength as usize, self.language)?;
        Ok(mnemonic)
    }
    
    /// Create a mnemonic from an existing phrase
    pub fn from_phrase<S: AsRef<str>>(&self, phrase: S) -> WalletResult<Mnemonic> {
        let mnemonic = Mnemonic::from_phrase(phrase.as_ref(), self.language)?;
        Ok(mnemonic)
    }
    
    /// Generate a seed from the mnemonic
    pub fn seed_from_mnemonic(&self, mnemonic: &Mnemonic) -> Zeroizing<Vec<u8>> {
        let passphrase = match &self.passphrase {
            Some(p) => p.as_str(),
            None => "",
        };
        
        // Generate the seed directly using the mnemonic's to_seed function
        let seed_bytes = mnemonic.to_seed(passphrase);
        
        Zeroizing::new(seed_bytes.to_vec())
    }
    
    /// Derive a key from the seed using the given path
    pub fn derive_key(&self, seed: &[u8], path: &str) -> WalletResult<Zeroizing<Vec<u8>>> {
        // Parse the derivation path
        let path = BIP32Path::from_str(path)
            .map_err(|e| crate::wallet::WalletError::InvalidKey(e.to_string()))?;
        
        // Derive the key
        let derived_key = derive_key_from_path(seed, &path, Curve::Ed25519)
            .map_err(|e| crate::wallet::WalletError::InvalidKey(e.to_string()))?;
        
        // Return the private key
        Ok(Zeroizing::new(derived_key.private_key().to_vec()))
    }
}

/// Various security utilities for working with mnemonics and seeds
pub struct MnemonicUtils;

impl MnemonicUtils {
    /// Validate a mnemonic phrase
    pub fn validate_mnemonic<S: AsRef<str>>(phrase: S, language: Language) -> bool {
        Mnemonic::validate(phrase.as_ref(), language)
    }
    
    /// Generate a secure passphrase
    pub fn generate_passphrase(length: usize) -> SecretString {
        use rand::{Rng, thread_rng};
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{}|;:,.<>?";
        
        let mut rng = thread_rng();
        let passphrase: String = (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        
        SecretString::new(passphrase)
    }
    
    /// Securely wipe memory containing sensitive data
    pub fn secure_wipe<T: Zeroize>(data: &mut T) {
        data.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mnemonic_generation() {
        let builder = MnemonicBuilder::new();
        let mnemonic = builder.generate().unwrap();
        
        // Check that we have a valid mnemonic
        assert!(MnemonicUtils::validate_mnemonic(mnemonic.phrase(), Language::English));
    }
    
    #[test]
    fn test_key_derivation() {
        let builder = MnemonicBuilder::new();
        let mnemonic = builder.generate().unwrap();
        let seed = builder.seed_from_mnemonic(&mnemonic);
        
        // Derive key using default path
        let key = builder.derive_key(&seed, DEFAULT_DERIVATION_PATH).unwrap();
        
        // Check that we got a valid key (32 bytes for Ed25519)
        assert_eq!(key.len(), 32);
    }
} 