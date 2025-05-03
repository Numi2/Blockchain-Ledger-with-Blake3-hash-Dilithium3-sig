use crate::types::{Address, Hash, Transaction};
use crate::wallet::{AccountInfo, AccountManager, WalletError, WalletResult, SignatureScheme};
use secrecy::SecretString;
use std::collections::HashMap;

/// TransactionBuilder helps with creating and signing transactions
pub struct TransactionBuilder {
    /// Transaction being built
    tx: Transaction,
    
    /// Account manager for signing
    account_manager: AccountManager,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new(account_manager: AccountManager) -> Self {
        Self {
            tx: Transaction::default(),
            account_manager,
        }
    }
    
    /// Set the sender address
    pub fn from(mut self, from: Address) -> Self {
        self.tx.from = from;
        self
    }
    
    /// Set the recipient address
    pub fn to(mut self, to: Address) -> Self {
        self.tx.to = Some(to);
        self
    }
    
    /// Set the transaction value
    pub fn value(mut self, value: u64) -> Self {
        self.tx.value = value;
        self
    }
    
    /// Set the gas limit
    pub fn gas_limit(mut self, gas_limit: u64) -> Self {
        self.tx.gas_limit = gas_limit;
        self
    }
    
    /// Set the gas price
    pub fn gas_price(mut self, gas_price: u64) -> Self {
        self.tx.gas_price = gas_price;
        self
    }
    
    /// Set the nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.tx.nonce = nonce;
        self
    }
    
    /// Set the data
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.tx.data = data;
        self
    }
    
    /// Sign the transaction with the given account
    pub fn sign(
        mut self,
        account_info: &AccountInfo,
        password: &SecretString,
    ) -> WalletResult<SignedTransaction> {
        // Ensure the from address matches the account
        if self.tx.from != account_info.address {
            return Err(WalletError::InvalidTransaction(
                "Transaction sender address doesn't match the signing account".to_string()
            ));
        }
        
        // Sign the transaction
        let signature = self.account_manager.sign_transaction(
            account_info,
            &self.tx,
            password,
        )?;
        
        // Create signed transaction
        let signed_tx = SignedTransaction {
            transaction: self.tx,
            signature,
            signature_scheme: account_info.signature_scheme,
        };
        
        Ok(signed_tx)
    }
}

/// SignedTransaction contains a transaction and its signature
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedTransaction {
    /// The transaction
    pub transaction: Transaction,
    
    /// The signature
    pub signature: Vec<u8>,
    
    /// The signature scheme used
    pub signature_scheme: SignatureScheme,
}

impl SignedTransaction {
    /// Get the transaction hash
    pub fn hash(&self) -> Hash {
        self.transaction.hash()
    }
    
    /// Serialize the transaction for transmission
    pub fn serialize(&self) -> WalletResult<Vec<u8>> {
        let serialized = serde_json::to_vec(self)
            .map_err(|e| WalletError::Other(format!("Failed to serialize transaction: {}", e)))?;
            
        Ok(serialized)
    }
    
    /// Deserialize a transaction
    pub fn deserialize(bytes: &[u8]) -> WalletResult<Self> {
        let tx = serde_json::from_slice(bytes)
            .map_err(|e| WalletError::Other(format!("Failed to deserialize transaction: {}", e)))?;
            
        Ok(tx)
    }
    
    /// Verify the transaction signature
    pub fn verify(&self, account_manager: &AccountManager) -> WalletResult<bool> {
        // Get the transaction hash (the message that was signed)
        let message = self.transaction.hash();
        
        // Verify the signature using the appropriate scheme
        account_manager.verify_signature(
            self.signature_scheme,
            &self.transaction.from, // This assumes Address is the same as PublicKey which may not be correct
            &message,
            &self.signature,
        )
    }
}

/// Transaction manager for handling transaction submission and tracking
pub struct TransactionManager {
    /// Account manager for signing transactions
    account_manager: AccountManager,
    
    /// Pending transactions by hash
    pending_transactions: HashMap<Hash, SignedTransaction>,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(account_manager: AccountManager) -> Self {
        Self {
            account_manager,
            pending_transactions: HashMap::new(),
        }
    }
    
    /// Create a new transaction builder
    pub fn create_transaction(&self) -> TransactionBuilder {
        TransactionBuilder::new(self.account_manager.clone())
    }
    
    /// Submit a transaction
    pub fn submit_transaction(&mut self, tx: SignedTransaction) -> WalletResult<Hash> {
        // Verify the transaction signature
        if !tx.verify(&self.account_manager)? {
            return Err(WalletError::InvalidTransaction("Invalid signature".to_string()));
        }
        
        // Get the transaction hash
        let hash = tx.hash();
        
        // Store in pending transactions
        self.pending_transactions.insert(hash, tx);
        
        // TODO: Actually submit to the network
        
        Ok(hash)
    }
    
    /// Get a pending transaction by hash
    pub fn get_pending_transaction(&self, hash: &Hash) -> Option<&SignedTransaction> {
        self.pending_transactions.get(hash)
    }
    
    /// Check if a transaction is pending
    pub fn is_transaction_pending(&self, hash: &Hash) -> bool {
        self.pending_transactions.contains_key(hash)
    }
    
    /// Remove a transaction from pending (e.g., when confirmed)
    pub fn remove_pending_transaction(&mut self, hash: &Hash) -> Option<SignedTransaction> {
        self.pending_transactions.remove(hash)
    }
} 