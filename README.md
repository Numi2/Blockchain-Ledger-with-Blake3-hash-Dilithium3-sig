# World Ledger Blockchain

World Ledger is a Rust-based blockchain implementation designed with security, minimalism, and scalability as its core principles. It features quantum-resistant cryptography and a UTXO-based state model combined with WebAssembly smart contracts.

## Core Design Principles

- **Security First**: Minimal attack surface through careful code design and small codebase
- **Quantum Resistance**: Dilithium signatures for long-term cryptographic security
- **Modular Architecture**: Clean separation of concerns with well-defined interfaces
- **Scalability**: Optimized for high throughput and low latency

## Implemented Components

### Core Components
- Storage system for blockchain data with RocksDB backend
- Wallet with BIP39 mnemonics and post-quantum Dilithium signatures
- Consensus mechanisms (PoW and PoS)
- WebAssembly smart contract execution system
- P2P networking with peer discovery and reputation management
- CLI for node and wallet operations
- Monitoring and telemetry infrastructure
- Testing framework

### Production Readiness Features

We've implemented the first four steps of a comprehensive 10-step production readiness plan:

#### 1. Core Functionality
- ✅ STARK proof generation/verification system for state transitions
- ✅ State bridge for integrating smart contract VM with UTXO state model
- ✅ UTXO state implementation with Merkle tree support

#### 2. Networking & P2P Enhancements
- ✅ Configurable block synchronization strategies (tip, headers-first, parallel, warp)
- ✅ Transaction propagation with rate limiting
- ✅ Enhanced peer reputation system with blacklisting and ban expiration

#### 3. Security Hardening
- ✅ Memory-hardened password hashing (Argon2id)
- ✅ Key rotation system for validator keys
- ✅ Security policy and vulnerability disclosure process

#### 4. Smart Contract System
- ✅ WebAssembly contract compiler toolchain with multiple language support
- ✅ Security verification and scanning for smart contracts
- ✅ Contract upgrade mechanism with governance controls

## Remaining Steps for Production

The following steps from our 10-step plan require completion before full production readiness:

#### 5. Scalability & Performance Optimization
- ⬜ Implement parallel transaction validation
- ⬜ Add state sharding
- ⬜ Optimize consensus for throughput

#### 6. Economics & Governance
- ⬜ Finalize economic model and gas pricing
- ⬜ Implement governance mechanism for protocol upgrades
- ⬜ Design and implement staking incentives

#### 7. Interoperability & Standards
- ⬜ Implement cross-chain communication protocols
- ⬜ Add support for blockchain standards
- ⬜ Build interoperability bridges

#### 8. Ecosystem & Developer Tools
- ⬜ Create comprehensive SDK for developers
- ⬜ Build block explorer and analytics dashboard
- ⬜ Develop testing and deployment frameworks

#### 9. Auditing & Verification
- ⬜ Complete third-party security audits
- ⬜ Formal verification of critical components
- ⬜ Stress testing and performance benchmarking

#### 10. Documentation & Release Process
- ⬜ Comprehensive documentation (API, deployment, operations)
- ⬜ Define release process and versioning strategy
- ⬜ Create upgrade paths and backward compatibility plans

## Getting Started

### Prerequisites
- Rust 1.70+ with nightly toolchain
- RocksDB 6.0+
- System dependencies required by the Rust crates

### Building from Source
```bash
# Clone the repository
git clone 
cd world-ledger

# Build in release mode
cargo build --release
```

### Running a Node
```bash
# Run a node in development mode
cargo run -- node start --dev

# Run a full node connected to the main network
cargo run -- node start --network main
```

### Using the Wallet
```bash
# Create a new wallet
cargo run -- wallet create

# Check balance
cargo run -- wallet balance

# Send transaction
cargo run -- wallet send --to <address> --amount <amount>
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Security

For security concerns, please refer to [SECURITY.md](docs/SECURITY.md).

## License

This project is licensed under either of
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
