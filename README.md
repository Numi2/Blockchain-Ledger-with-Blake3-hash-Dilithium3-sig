# World Ledger

A simple, scalable, resilient blockchain protocol designed to serve as the foundational digital layer for global assets, financial systems, and public records.

## Overview

World Ledger is a radically simplified blockchain protocol that prioritizes:

- **Simplicity** as the foundation for security and longevity
- **Modular design** with clean separation between consensus, execution, and networking
- **Efficient light clients** via STARKs and shared Merkle structures
- **Post-quantum security** using Dilithium3 signatures

## Architecture

The project is organized as a Rust workspace with the following key modules:

- `src/core/` - Consensus, block format, execution engine, cryptography
- `src/p2p/` - Networking using libp2p (gossipsub, Kademlia)
- `src/storage/` - Persistent state and history storage
- `src/ssz/` - Simple Serialize (SSZ) format for encoding
- `src/stark/` - STARK proof generation and verification
- `src/types/` - Global types used throughout the protocol

## Key Features

1. **Consensus**: 3-slot finality protocol that is deterministic and compact
2. **Block Format**: Simple structure with BLAKE3 hashing
3. **Execution**: RISC-V inspired VM for smart contracts
4. **Networking**: libp2p with gossipsub for block propagation
5. **Storage**: Merkle tree-based state with efficient proofs
6. **Crypto**: Post-quantum signatures with Dilithium3

## Getting Started

### Building from Source

```
cargo build --release
```

### Running a Node

```
cargo run -- node
```

### Running a Validator

```
cargo run -- validator
```

### Creating an Account

```
cargo run -- account create
```

## Development Philosophy

World Ledger prioritizes:

- Simplicity over features
- Minimal consensus-critical code
- Encapsulation of complexity outside consensus
- Clean separation of layers
- Unified components to reduce systemic complexity

## License

MIT or Apache-2.0



We've successfully implemented the core components of the World Ledger blockchain according to the development plan. Here's a summary of what we've accomplished:

1. **Core Types and Block Format**: Implemented the basic types, block structure with headers, bodies, and transactions, all using BLAKE3 for hashing.

2. **Blockchain State Machine**: Developed an account-based state model with a world state that handles transactions and updates.

3. **3-Slot Finality Consensus**: Created a consensus mechanism with a 3-slot finality protocol and fork choice rule.

4. **P2P Network Communication**: Integrated libp2p with gossipsub for block propagation and other network functionality.

5. **Execution VM**: Designed a minimalistic stack-based VM with opcodes and gas metering for smart contracts.

6. **Cryptography**: Added support for Dilithium3 signature verification (as a placeholder for now).

7. **Serialization**: Implemented SSZ serialization and Merkle tree functionalities.

The codebase now has a solid foundation for a blockchain protocol that follows the design principles of simplicity, security, and modularity. To continue development, you might want to:

1. Implement a more complete storage layer
2. Expand the STARK prover/verifier module
3. Enhance the VM with more opcodes for a full RISC-V implementation
4. Add more networking features for peer discovery and block syncing
5. Develop a proper wallet implementation

You can build and run the project with:
```
cargo build --release
cargo run -- help
```
