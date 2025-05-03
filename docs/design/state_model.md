# State Model Design

## Overview

World Ledger uses a UTXO-based state model for maximum simplicity and auditability.

## Merkle Structure

The state is represented as a binary Merkle tree using BLAKE3 hashes, enabling efficient light client proofs.

## STARK Integration

STARK proofs will be used to represent consensus state transitions, enabling efficient verification.
