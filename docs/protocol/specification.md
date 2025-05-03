# World Ledger Protocol Specification

## Table of Contents
1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [Cryptography](#cryptography)
4. [Block Structure](#block-structure)
5. [State Model](#state-model)
6. [Consensus](#consensus)
7. [Networking](#networking)
8. [Smart Contracts](#smart-contracts)
9. [Light Clients](#light-clients)
10. [Security Considerations](#security-considerations)

## Overview
World Ledger is a blockchain protocol designed to serve as a secure, scalable, and quantum-resistant foundation for global assets, financial systems, and public records. This document specifies the core protocol design, data structures, cryptography, and consensus mechanisms.

## Design Principles
The World Ledger protocol is guided by the following principles:
- **Simplicity** as the foundation for security and longevity
- **Modular design** with clean separation between consensus, execution, and networking
- **Efficient light clients** via STARKs and shared Merkle structures
- **Post-quantum security** using Dilithium3 signatures

## Cryptography
### Hash Functions
- Primary hash function: **BLAKE3**
  - Used for transaction hashing, Merkle trees, and message authentication
  - Provides high performance and cryptographic strength

### Digital Signatures
- Primary signature scheme: **Dilithium3**
  - Post-quantum secure signature algorithm
  - NIST level 3 security (equivalent to AES-192)
  - Signature size: ~2.7 KB
  - Public key size: ~1.5 KB
  
- Optional legacy signature scheme: **Ed25519**
  - For backwards compatibility and reduced size
  - Signature size: 64 bytes
  - Public key size: 32 bytes

### Merkle Trees
- Binary Merkle trees with BLAKE3 for all state commitments
- Efficient inclusion proofs for light clients

## Block Structure
### Block Header
```rust
pub struct BlockHeader {
    pub version: u32,                // Protocol version
    pub prev_hash: Hash,             // Previous block hash
    pub height: u64,                 // Block height
    pub timestamp: u64,              // Unix timestamp
    pub state_root: Hash,            // UTXO set Merkle root
    pub transactions_root: Hash,     // Merkle root of transactions
    pub receipts_root: Hash,         // Merkle root of transaction receipts
    pub difficulty: u64,             // PoW difficulty target
    pub nonce: u64,                  // PoW nonce
    pub validator: Address,          // Block producer address
    pub signature: Signature,        // Block signature (Dilithium3)
}
```

### Block Body
```rust
pub struct BlockBody {
    pub transactions: Vec<Transaction>,  // List of transactions
}
```

### Transaction
```rust
pub struct Transaction {
    pub inputs: Vec<TxIn>,           // UTXO inputs
    pub outputs: Vec<TxOut>,         // UTXO outputs
    pub from: Address,               // Sender address
    pub to: Option<Address>,         // Recipient (None for contract creation)
    pub value: u64,                  // Transfer amount
    pub gas_limit: u64,              // Maximum gas
    pub gas_price: u64,              // Gas price in smallest units
    pub nonce: u64,                  // Sender's nonce
    pub data: Vec<u8>,               // Contract data or memo
    pub signature: Signature,        // Transaction signature
}
```

## State Model
World Ledger uses a UTXO-based state model for simplicity and auditability.

### UTXO
```rust
pub struct UTXO {
    pub id: Hash,                    // Unique ID (hash of tx + output index)
    pub tx_hash: Hash,               // Transaction hash
    pub index: u32,                  // Output index in transaction
    pub value: u64,                  // Value in smallest units
    pub owner: Address,              // Owner address
    pub created_at: u64,             // Block height when created
    pub spent: bool,                 // Is this UTXO spent
    pub spent_at: Option<u64>,       // Block height when spent
}
```

### State Transitions
1. Transactions consume UTXOs as inputs
2. New UTXOs are created as outputs
3. The state root is a Merkle root of all unspent UTXOs
4. Each block transitions the state by applying its transactions

## Consensus
World Ledger supports both Proof-of-Work and Proof-of-Stake consensus:

### Proof-of-Work
- BLAKE3-based mining algorithm
- Difficulty adjustment every 2016 blocks (targeting 10-minute block time)
- Block rewards halving every 210,000 blocks

### Proof-of-Stake
- Uses a 3-slot finality protocol
- Validators stake tokens to participate
- BFT-style consensus with Byzantine fault tolerance
- Slashing for equivocation or unavailability

## Networking
### P2P Protocol
- Built on libp2p framework
- Gossipsub for block and transaction propagation
- Kademlia DHT for peer discovery
- Request-response pattern for block sync

### Message Types
1. **Block**: Propagate new blocks
2. **Transaction**: Propagate new transactions
3. **GetBlocks**: Request blocks by height range
4. **GetState**: Request state (UTXOs) by address

## Smart Contracts
### WebAssembly VM
- Smart contracts compiled to WebAssembly
- Metered execution with gas
- Deterministic behavior across platforms

### Runtime Environment
- Contract storage (key-value store)
- Inter-contract calls
- Event emission
- Gas metering for all operations

### ABI
- Simple and deterministic encoding/decoding
- Static typing
- Support for standard primitive types

## Light Clients
Light clients can efficiently verify the blockchain without downloading all data:

1. Light clients need only block headers
2. UTXO inclusion proofs allow verification of specific UTXOs
3. STARK proofs enable efficient verification of state transitions

## Security Considerations
- All cryptographic operations use constant-time algorithms
- Post-quantum security through Dilithium3 signatures
- Comprehensive fuzzing and formal verification of critical components
- Strict validation of all inputs and state transitions
- Memory hardened hashing for key derivation 