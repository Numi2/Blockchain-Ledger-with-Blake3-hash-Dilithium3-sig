use crate::core::block::Transaction;
use crate::storage::{Column, Database, StorageError, StorageResult};
use crate::types::{Address, ExecutionStatus, Hash};
use crate::ssz::{serialize, deserialize};
use std::collections::HashMap;

/// Receipt stores the result of a transaction execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionReceipt {
    /// Transaction hash
    pub transaction_hash: Hash,
    
    /// Block hash where this transaction was included
    pub block_hash: Hash,
    
    /// Block slot where this transaction was included
    pub block_slot: u64,
    
    /// Index of this transaction in the block
    pub transaction_index: u32,
    
    /// Sender address
    pub from: Address,
    
    /// Recipient address (if any)
    pub to: Option<Address>,
    
    /// Execution status
    pub status: ExecutionStatus,
    
    /// Gas used by this transaction
    pub gas_used: u64,
    
    /// Logs emitted during execution
    pub logs: Vec<Log>,
    
    /// Contract address created (if any)
    pub contract_created: Option<Address>,
}

/// Log represents an event emitted during transaction execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Log {
    /// Address that emitted this log
    pub address: Address,
    
    /// Topics (indexed parameters)
    pub topics: Vec<Hash>,
    
    /// Data (non-indexed parameters)
    pub data: Vec<u8>,
}

/// TransactionStorage provides functionality for storing and retrieving transactions
pub struct TransactionStorage {
    db: Database,
}

impl TransactionStorage {
    /// Create a new TransactionStorage with the given database
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    /// Store a transaction
    pub fn store_transaction(&self, tx: &Transaction) -> StorageResult<()> {
        let tx_hash = tx.hash();
        let tx_bytes = serialize(tx)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(Column::Transactions, &tx_hash, &tx_bytes)
    }
    
    /// Store a transaction receipt
    pub fn store_receipt(&self, receipt: &TransactionReceipt) -> StorageResult<()> {
        let receipt_bytes = serialize(receipt)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(Column::Receipts, &receipt.transaction_hash, &receipt_bytes)
    }
    
    /// Get a transaction by hash
    pub fn get_transaction(&self, hash: &Hash) -> StorageResult<Option<Transaction>> {
        let tx_bytes = match self.db.get(Column::Transactions, hash)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        let tx = deserialize(&tx_bytes)
            .map_err(|e| StorageError::Serialization(e))?;
        
        Ok(Some(tx))
    }
    
    /// Get a transaction receipt by transaction hash
    pub fn get_receipt(&self, tx_hash: &Hash) -> StorageResult<Option<TransactionReceipt>> {
        let receipt_bytes = match self.db.get(Column::Receipts, tx_hash)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        let receipt = deserialize(&receipt_bytes)
            .map_err(|e| StorageError::Serialization(e))?;
        
        Ok(Some(receipt))
    }
    
    /// Get all transactions associated with an address
    pub fn get_transactions_by_address(&self, address: &Address) -> StorageResult<Vec<Transaction>> {
        let mut transactions = Vec::new();
        
        // This is inefficient as it scans all transactions
        // In a production system, we would maintain an index of address -> tx hashes
        let iter = self.db.iter(Column::Transactions)?;
        
        for (_, tx_bytes) in iter {
            let tx: Transaction = match deserialize(&tx_bytes) {
                Ok(tx) => tx,
                Err(_) => continue, // Skip invalid transactions
            };
            
            if tx.from == *address || (tx.to.is_some() && tx.to.unwrap() == *address) {
                transactions.push(tx);
            }
        }
        
        Ok(transactions)
    }
    
    /// Get all receipts associated with an address
    pub fn get_receipts_by_address(&self, address: &Address) -> StorageResult<Vec<TransactionReceipt>> {
        let mut receipts = Vec::new();
        
        // This is inefficient as it scans all receipts
        // In a production system, we would maintain an index of address -> receipt hashes
        let iter = self.db.iter(Column::Receipts)?;
        
        for (_, receipt_bytes) in iter {
            let receipt: TransactionReceipt = match deserialize(&receipt_bytes) {
                Ok(receipt) => receipt,
                Err(_) => continue, // Skip invalid receipts
            };
            
            if receipt.from == *address || (receipt.to.is_some() && receipt.to.unwrap() == *address) {
                receipts.push(receipt);
            }
        }
        
        Ok(receipts)
    }
    
    /// Get all receipts in a block
    pub fn get_receipts_by_block(&self, block_hash: &Hash) -> StorageResult<Vec<TransactionReceipt>> {
        let mut receipts = Vec::new();
        
        // This is inefficient as it scans all receipts
        // In a production system, we would maintain an index of block hash -> receipt hashes
        let iter = self.db.iter(Column::Receipts)?;
        
        for (_, receipt_bytes) in iter {
            let receipt: TransactionReceipt = match deserialize(&receipt_bytes) {
                Ok(receipt) => receipt,
                Err(_) => continue, // Skip invalid receipts
            };
            
            if receipt.block_hash == *block_hash {
                receipts.push(receipt);
            }
        }
        
        // Sort by transaction index
        receipts.sort_by_key(|r| r.transaction_index);
        
        Ok(receipts)
    }
    
    /// Delete a transaction and its receipt
    pub fn delete_transaction(&self, hash: &Hash) -> StorageResult<()> {
        let mut batch = self.db.batch();
        batch.delete(Column::Transactions, hash)
             .delete(Column::Receipts, hash);
        
        batch.write()
    }
    
    /// Check if a transaction exists
    pub fn transaction_exists(&self, hash: &Hash) -> StorageResult<bool> {
        self.db.exists(Column::Transactions, hash)
    }
    
    /// Get the latest nonce for an address
    pub fn get_latest_nonce(&self, address: &Address) -> StorageResult<u64> {
        let transactions = self.get_transactions_by_address(address)?;
        
        let max_nonce = transactions.iter()
            .filter(|tx| tx.from == *address)
            .map(|tx| tx.nonce)
            .max()
            .unwrap_or(0);
        
        Ok(max_nonce)
    }
} 