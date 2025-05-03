use crate::types::{Address, Hash, Signature};
use serde::{Serialize, Deserialize};

/// Represents an unspent transaction output
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utxo {
    /// Transaction hash that created this UTXO
    pub tx_hash: Hash,
    
    /// Output index within the transaction
    pub output_index: u32,
    
    /// Value of this UTXO
    pub value: u64,
    
    /// Owner address
    pub address: Address,
    
    /// Locktime (0 means immediately spendable)
    pub locktime: u64,
}

/// Represents a transaction input (spending a UTXO)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxInput {
    /// Reference to the UTXO being spent
    pub utxo_ref: UtxoRef,
    
    /// Signature authorizing the spend
    pub signature: Signature,
}

/// Reference to a UTXO to be spent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtxoRef {
    /// Transaction hash containing the UTXO
    pub tx_hash: Hash,
    
    /// Output index within the transaction
    pub output_index: u32,
}

/// Represents a transaction output creating a new UTXO
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOutput {
    /// Value of the output
    pub value: u64,
    
    /// Owner address
    pub address: Address,
    
    /// Locktime (0 means immediately spendable)
    pub locktime: u64,
}

/// UTXO-based transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtxoTransaction {
    /// Transaction inputs
    pub inputs: Vec<TxInput>,
    
    /// Transaction outputs
    pub outputs: Vec<TxOutput>,
    
    /// Transaction version
    pub version: u32,
    
    /// Locktime
    pub locktime: u64,
}
