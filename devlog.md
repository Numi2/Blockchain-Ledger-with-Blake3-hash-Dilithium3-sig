World Ledger Development Log
    
     Progress Log
    
     Step 1: Core Block Format, Header, and Hashing (COMPLETE)
    What was accomplished:
    - Defined canonical types in src/types/types.rs:

          `Hash`: 32-byte array for BLAKE3
          `Address`: 32-byte array (pubkey hash)
          `Signature`: Flexible vector
          `Slot`: u64
                - Implemented the core block data structures in
    `src/core/block/mod.rs`:
          `BlockHeader`: Minimal, consensus-critical info (parent hash, roots,
     slot, proposer, signature, timestamp).
          `BlockBody`: Opaque transaction list, optional extra data.
          `Block`: Header  body.
                - Added `BlockHeader::hash()` method:
          Computes the BLAKE3 hash of the SSZ-serialized header.
                - Structures are set up for Serde (`Serialize`, `Deserialize`).
                - All implementations use only the atomic types and do not
    expose cryptographic/storage/merkle logic in consensus paths.
        

Yes — here is the clarified state model and block format for the World Ledger, followed by a confirmation that you should proceed with initial Rust code stubs in src/core/state/.

⸻

✅ Block Format Specification

This blockchain is UTXO-based, similar to Bitcoin, to maximize simplicity and auditability. Each block contains:

📦 BlockHeader

pub struct BlockHeader {
    pub parent_hash: Hash,
    pub state_root: Hash,
    pub transactions_root: Hash,
    pub timestamp: u64,
    pub height: u64,
    pub nonce: u64,
}

📦 Block

pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}



⸻

✅ State Model (UTXO Set)

The state is a hash map of UTXOs, where each UTXO is:

pub struct UTXO {
    pub value: u64,
    pub owner: Address,
    pub spent: bool,
}

The global state at any block height is:

pub type State = HashMap<UTXOId, UTXO>;

	•	UTXOId is a BLAKE3 hash of the (txid, index) pair.
	•	The state_root in the block header is the Merkle root of all unspent UTXOs.

⸻
 Transaction Format

pub struct Transaction {
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    pub signature: Signature,
}

pub struct TxIn {
    pub utxo_id: Hash,
}

pub struct TxOut {
    pub value: u64,
    pub recipient: Address,
}

	•	Transactions consume UTXOs as inputs and create new UTXOs as outputs.
	•	Validity rules: inputs must exist, be unspent, and signature must match owner.

⸻



You can start with:
	•	state.rs (managing the UTXO set)
	•	utxo.rs (data structures)
	•	apply_transaction() and apply_block() functions
	•	merkle.rs (Merkle tree logic for computing state root)

    
    ---
    
     Next Steps
    
     Step 2: Minimal Blockchain State Machine
    - Implement a minimal deterministic state machine for managing state
    transitions.
    - File: src/core/state/
    - Should:
- RISC-V, minimizing consensus-critical code, and the core data structure used for state representation. use one single shared tree structure across the execution layer and the consensus layer, like a Blake3 binary tree

          Take a block, apply to state, update state root.
          Expose interface for state hashing (using the shared Merkle/BLAKE3
    scheme).
          Remain minimal and consensus-focused.

Yes — here is the clarified state model and block format for the World Ledger, followed by a confirmation that you should proceed with initial Rust code stubs in src/core/state/.

⸻

Block Format Specification

This blockchain is UTXO-based, similar to Bitcoin, to maximize simplicity and auditability. Each block contains:

📦 BlockHeader

pub struct BlockHeader {
    pub parent_hash: Hash,
    pub state_root: Hash,
    pub transactions_root: Hash,
    pub timestamp: u64,
    pub height: u64,
    pub nonce: u64,
}

📦 Block

pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}



⸻

✅ State Model (UTXO Set)

The state is a hash map of UTXOs, where each UTXO is:

pub struct UTXO {
    pub value: u64,
    pub owner: Address,
    pub spent: bool,
}

The global state at any block height is:

pub type State = HashMap<UTXOId, UTXO>;

	•	UTXOId is a BLAKE3 hash of the (txid, index) pair.
	•	The state_root in the block header is the Merkle root of all unspent UTXOs.

⸻

✅ Transaction Format

pub struct Transaction {
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    pub signature: Signature,
}

pub struct TxIn {
    pub utxo_id: Hash,
}

pub struct TxOut {
    pub value: u64,
    pub recipient: Address,
}

	•	Transactions consume UTXOs as inputs and create new UTXOs as outputs.
	•	Validity rules: inputs must exist, be unspent, and signature must match owner.

⸻

Yes, please proceed with initial Rust code stubs in src/core/state/.

You can start with:
	•	state.rs (managing the UTXO set)
	•	utxo.rs (data structures)
	•	apply_transaction() and apply_block() functions
	•	merkle.rs (Merkle tree logic for computing state root)
- 
- 
        

     Further Steps (Roadmap)
    - Step 3: 3-slot finality consensus (in core/consensus)
    - Step 4: P2P block propagation (in p2p/)
    - Step 5: RISC-V execution VM (in core/execution)
    - Step 6: Validator sigs with dilithium3
    - Step 7: Complete serdessz wiring & test all layers
    
    ---
    
    Always keep consensus-critical code simple, testable, and encapsulated!
   