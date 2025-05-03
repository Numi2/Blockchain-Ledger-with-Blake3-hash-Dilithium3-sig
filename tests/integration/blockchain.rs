use super::*;
use wl::core::block::{Block, BlockHeader, BlockBody};
use wl::types::{Hash, Transaction};
use wl::consensus::pow::ProofOfWork;
use wl::consensus::ConsensusConfig;
use wl::consensus::types::{Consensus, ConsensusBlockHeader};
use wl::core::mempool::MemPool;
use wl::core::state::WorldState;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Test block creation and validation
#[test]
fn test_block_creation_and_validation() {
    let config = setup();
    
    // Create a block
    let prev_hash = [0u8; 32]; // Genesis block has zero prev_hash
    let height = 1;
    let state_root = config.world_state.state_root();
    
    // Create transaction
    let tx = config.create_test_transaction(0, 1, 1_000_000_000_000_000_000); // 1 token
    
    // Create test block
    let block = Block::new(
        prev_hash,
        height,
        1, // Slot
        state_root,
        vec![tx.clone()],
        [0u8; 20], // Producer address
        1, // Difficulty
    );
    
    // Calculate block hash
    let block_hash = block.hash();
    
    // Ensure hash is not zero
    assert!(block_hash != [0u8; 32]);
    
    // Create consensus block header
    let consensus_header = ConsensusBlockHeader {
        hash: block_hash,
        prev_hash,
        height,
        slot: 1,
        timestamp: block.header.timestamp,
        state_root,
        transactions_root: block.header.transactions_root,
    };
    
    // Create consensus
    let consensus_config = ConsensusConfig::default();
    let mut consensus = ProofOfWork::new(consensus_config, 0); // Genesis time = 0
    
    // Validate block
    let validation = consensus.validate_block(&consensus_header);
    assert!(matches!(validation, wl::consensus::types::BlockValidationResult::Valid));
    
    // Process block
    assert!(consensus.process_block(&consensus_header).is_ok());
    
    // Check consensus state
    let status = consensus.status();
    assert_eq!(status.current_height, height);
    
    teardown(config);
}

/// Test transaction processing
#[test]
fn test_transaction_processing() {
    let config = setup();
    
    // Create MemPool
    let mempool = MemPool::new(config.world_state.clone(), 1000);
    
    // Create transaction
    let tx = config.create_test_transaction(0, 1, 1_000_000_000_000_000_000); // 1 token
    
    // Set up account balance for the sender
    {
        let mut world_state = WorldState::new(config.db.clone());
        let account = world_state.get_or_create_account(&tx.from);
        account.balance = 10_000_000_000_000_000_000; // 10 tokens
        world_state.commit().expect("Failed to commit state");
    }
    
    // Add transaction to mempool
    assert!(mempool.add_transaction(tx.clone()).is_ok());
    
    // Get transactions for a new block
    let block_txs = mempool.get_transactions(10);
    assert!(!block_txs.is_empty());
    assert_eq!(block_txs[0].hash(), tx.hash()); // Should be our transaction
    
    // Create a block with this transaction
    let prev_hash = [0u8; 32]; // Genesis block has zero prev_hash
    let height = 1;
    let state_root = config.world_state.state_root();
    
    let block = Block::new(
        prev_hash,
        height,
        1, // Slot
        state_root,
        block_txs.clone(),
        [0u8; 20], // Producer address
        1, // Difficulty
    );
    
    // Apply transaction to state
    {
        let mut world_state = WorldState::new(config.db.clone());
        
        // Get sender account
        let sender = world_state.get_or_create_account(&tx.from);
        let sender_balance_before = sender.balance;
        
        // Get recipient account
        let recipient = world_state.get_or_create_account(&tx.to.unwrap());
        let recipient_balance_before = recipient.balance;
        
        // Apply transaction
        sender.balance -= tx.value;
        sender.nonce += 1;
        recipient.balance += tx.value;
        
        // Commit changes
        world_state.commit().expect("Failed to commit state");
        
        // Verify balances
        let sender_after = world_state.get_account(&tx.from).unwrap();
        let recipient_after = world_state.get_account(&tx.to.unwrap()).unwrap();
        
        assert_eq!(sender_after.balance, sender_balance_before - tx.value);
        assert_eq!(recipient_after.balance, recipient_balance_before + tx.value);
        assert_eq!(sender_after.nonce, 1);
    }
    
    teardown(config);
}

/// Test blockchain multi-block operations
#[test]
fn test_blockchain_multi_block() {
    let config = setup();
    
    // Create initial state
    let mut world_state = WorldState::new(config.db.clone());
    
    // Set up account balances
    {
        let account1 = world_state.get_or_create_account(&hex::decode(&config.addresses[0][2..]).unwrap().try_into().unwrap());
        account1.balance = 10_000_000_000_000_000_000; // 10 tokens
        
        let account2 = world_state.get_or_create_account(&hex::decode(&config.addresses[1][2..]).unwrap().try_into().unwrap());
        account2.balance = 0; // 0 tokens
        
        world_state.commit().expect("Failed to commit state");
    }
    
    // Create consensus
    let consensus_config = ConsensusConfig::default();
    let mut consensus = ProofOfWork::new(consensus_config, 0); // Genesis time = 0
    
    // Create mempool
    let mempool = Arc::new(RwLock::new(MemPool::new(Arc::new(world_state), 1000)));
    
    // Create blocks (simulate a blockchain)
    let mut prev_hash = [0u8; 32]; // Genesis
    
    for height in 1..=5 {
        // Create transaction for this block
        let tx = config.create_test_transaction(0, 1, 1_000_000_000_000_000_000 / 5); // 0.2 tokens
        
        // Add to mempool
        mempool.write().unwrap().add_transaction(tx.clone()).expect("Failed to add tx to mempool");
        
        // Get transactions for this block
        let txs = mempool.read().unwrap().get_transactions(10);
        
        // Create a block
        let mut block = Block::new(
            prev_hash,
            height,
            height as u64, // Slot
            [0u8; 32], // Will compute correct state root below
            txs,
            [0u8; 20], // Producer address
            consensus.status().current_difficulty.unwrap(), // Get current difficulty
        );
        
        // Mine the block (find a valid nonce)
        let mut nonce = 0u64;
        loop {
            block.header.nonce = nonce;
            let hash = consensus.calculate_mining_hash(&ConsensusBlockHeader {
                hash: [0u8; 32], // Placeholder
                prev_hash: block.header.prev_hash,
                height: block.header.height,
                slot: block.header.slot,
                timestamp: block.header.timestamp,
                state_root: block.header.state_root,
                transactions_root: block.header.transactions_root,
            }, nonce);
            
            if consensus.check_pow(&hash, consensus.status().current_difficulty.unwrap()) {
                // Found a valid block!
                break;
            }
            
            nonce += 1;
        }
        
        // Calculate block hash
        let block_hash = block.hash();
        
        // Process block in consensus
        let header = ConsensusBlockHeader {
            hash: block_hash,
            prev_hash: block.header.prev_hash,
            height: block.header.height,
            slot: block.header.slot,
            timestamp: block.header.timestamp,
            state_root: block.header.state_root,
            transactions_root: block.header.transactions_root,
        };
        
        assert!(consensus.process_block(&header).is_ok());
        
        // Update prev_hash for next block
        prev_hash = block_hash;
        
        // Remove processed transactions from mempool
        let tx_hashes: Vec<Hash> = block.body.transactions.iter().map(|tx| tx.hash()).collect();
        mempool.write().unwrap().remove_transactions(&tx_hashes);
    }
    
    // Verify consensus state
    let status = consensus.status();
    assert_eq!(status.current_height, 5);
    
    // Verify final balances
    let final_world_state = WorldState::new(config.db.clone());
    
    let account1 = final_world_state.get_account(&hex::decode(&config.addresses[0][2..]).unwrap().try_into().unwrap()).unwrap();
    let account2 = final_world_state.get_account(&hex::decode(&config.addresses[1][2..]).unwrap().try_into().unwrap()).unwrap();
    
    // Account 1 sent 5 * 0.2 = 1 token
    assert_eq!(account1.balance, 9_000_000_000_000_000_000); // 9 tokens
    
    // Account 2 received 5 * 0.2 = 1 token
    assert_eq!(account2.balance, 1_000_000_000_000_000_000); // 1 token
    
    // Account 1 nonce should be 5
    assert_eq!(account1.nonce, 5);
    
    teardown(config);
} 