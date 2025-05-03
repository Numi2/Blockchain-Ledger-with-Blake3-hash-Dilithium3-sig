use crate::types::types::Hash;

/// Represents an unspent transaction output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub txid: Hash,
    pub index: u32,
    pub value: u64,
    pub address: [u8; 32], // Typically the recipient's public key hash 
or address
}

/// A Transaction Input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxInput {
    pub txid: Hash,
    pub index: u32,
    // In production: signature, etc.
}

/// A Transaction Output (for convenience)
pub type TxOutput = Utxo;

/// Transaction: Basic structure, extendable for signatures/witness in 
production
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    // For production: locktime, version, etc.
}
