// Genesis block generation script
use world_ledger::{
    core::{Block, BlockHeader, BlockBody, UTXOState, State},
    types::{Transaction, Hash, Address, TxOut},
    wallet::{AccountManager, AccountInfo, SignatureScheme},
    config::GenesisConfig,
};
use std::path::PathBuf;
use std::fs;
use std::io::{self, Write};
use chrono::{Utc, DateTime};
use rand::thread_rng;
use std::collections::HashMap;
use secrecy::SecretString;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};

/// Genesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisParams {
    /// Genesis timestamp (Unix timestamp in seconds)
    pub timestamp: u64,
    
    /// Genesis difficulty
    pub difficulty: u64,
    
    /// Genesis nonce
    pub nonce: u64,
    
    /// Initial allocations (address -> amount)
    pub allocations: HashMap<String, u64>,
    
    /// Number of validator addresses to generate
    pub validator_count: usize,
    
    /// Genesis block version
    pub version: u32,
    
    /// Signature scheme for genesis validators
    pub signature_scheme: String,
}

impl Default for GenesisParams {
    fn default() -> Self {
        Self {
            timestamp: Utc::now().timestamp() as u64,
            difficulty: 1000, // Initial difficulty
            nonce: 0,
            allocations: HashMap::new(),
            validator_count: 10,
            version: 1,
            signature_scheme: "dilithium3".to_string(),
        }
    }
}

/// Command line arguments
#[derive(Debug, StructOpt)]
#[structopt(name = "genesis", about = "Generate a genesis block")]
struct Args {
    /// Path to genesis configuration file
    #[structopt(short, long, parse(from_os_str))]
    config: Option<PathBuf>,
    
    /// Output directory
    #[structopt(short, long, parse(from_os_str), default_value = ".")]
    output: PathBuf,
    
    /// Create a new default configuration
    #[structopt(long)]
    create_config: bool,
}

/// Generate a genesis block from parameters
fn generate_genesis(params: &GenesisParams, output_dir: &PathBuf) -> Result<(), String> {
    println!("Generating genesis block...");
    
    // Create keystore for validator keys
    let keystore_dir = output_dir.join("keystore");
    fs::create_dir_all(&keystore_dir).map_err(|e| e.to_string())?;
    
    let keystore = world_ledger::wallet::KeyStore::new(keystore_dir)
        .map_err(|e| format!("Failed to create keystore: {:?}", e))?;
    
    let mut account_manager = world_ledger::wallet::AccountManager::new(keystore)
        .map_err(|e| format!("Failed to create account manager: {:?}", e))?;
    
    // Create UTXOs for initial allocations
    let mut utxo_state = UTXOState::new();
    let mut transactions = Vec::new();
    let mut validator_addresses = Vec::new();
    
    // Create validator accounts
    println!("Generating {} validator accounts...", params.validator_count);
    for i in 0..params.validator_count {
        let name = format!("validator_{}", i);
        let password = SecretString::new("genesis".to_string()); // Not secure, but for genesis only
        
        // Create account with appropriate signature scheme
        let account_info = match params.signature_scheme.as_str() {
            "dilithium3" => {
                account_manager.create_dilithium_account(
                    name, 
                    &password, 
                    SignatureScheme::Dilithium3
                )
            },
            "ed25519" => {
                account_manager.create_ed25519_account(name, &password)
            },
            _ => return Err(format!("Unknown signature scheme: {}", params.signature_scheme)),
        }.map_err(|e| format!("Failed to create account: {:?}", e))?;
        
        let address_bytes = account_info.address;
        let mut address = [0u8; 20];
        address.copy_from_slice(&address_bytes);
        
        validator_addresses.push(address);
    }
    
    // Create initial allocation transactions
    println!("Creating initial allocations...");
    for (address_str, amount) in &params.allocations {
        // Parse address
        let address_bytes = hex::decode(address_str.strip_prefix("0x").unwrap_or(address_str))
            .map_err(|e| format!("Invalid address: {}", e))?;
            
        if address_bytes.len() != 20 {
            return Err(format!("Invalid address length: {}", address_bytes.len()));
        }
        
        let mut address = [0u8; 20];
        address.copy_from_slice(&address_bytes);
        
        // Create output
        let output = TxOut {
            value: *amount,
            recipient: address,
        };
        
        // Create coin generation transaction (no inputs, just outputs)
        let tx = Transaction {
            from: [0u8; 20], // Zero address (coin generation)
            to: Some(address),
            value: *amount,
            gas_limit: 0,
            gas_price: 0,
            nonce: 0,
            data: Vec::new(),
            inputs: Vec::new(),
            outputs: vec![output],
        };
        
        // Apply to state
        utxo_state.apply_transaction(&tx)
            .map_err(|e| format!("Failed to apply transaction: {}", e))?;
            
        transactions.push(tx);
    }
    
    // Compute state root
    let state_root = utxo_state.state_root();
    
    // Create Merkle root of transactions
    let transactions_root = compute_transactions_root(&transactions);
    
    // Create block header
    let header = BlockHeader {
        version: params.version,
        prev_hash: [0u8; 32], // Zero hash for genesis
        height: 0,
        timestamp: params.timestamp,
        state_root,
        transactions_root,
        receipts_root: [0u8; 32], // Empty for genesis
        difficulty: params.difficulty,
        nonce: params.nonce,
        validator: [0u8; 20], // Zero address for genesis
        signature: Vec::new(), // No signature for genesis
    };
    
    // Create block body
    let body = BlockBody {
        transactions,
    };
    
    // Create the block
    let genesis_block = Block {
        header,
        body,
    };
    
    // Serialize the genesis block
    let genesis_data = bincode::serialize(&genesis_block)
        .map_err(|e| format!("Failed to serialize genesis block: {}", e))?;
    
    // Write to file
    let output_file = output_dir.join("genesis.bin");
    fs::write(&output_file, &genesis_data)
        .map_err(|e| format!("Failed to write genesis file: {}", e))?;
    
    // Also write as JSON for readability
    let genesis_json = serde_json::to_string_pretty(&genesis_block)
        .map_err(|e| format!("Failed to serialize genesis to JSON: {}", e))?;
        
    let output_json = output_dir.join("genesis.json");
    fs::write(&output_json, genesis_json)
        .map_err(|e| format!("Failed to write genesis JSON: {}", e))?;
    
    // Write validator addresses
    let validator_json = serde_json::to_string_pretty(&validator_addresses)
        .map_err(|e| format!("Failed to serialize validator addresses: {}", e))?;
        
    let validators_file = output_dir.join("validators.json");
    fs::write(&validators_file, validator_json)
        .map_err(|e| format!("Failed to write validators file: {}", e))?;
    
    println!("Genesis files written to:");
    println!("  - {}", output_file.display());
    println!("  - {}", output_json.display());
    println!("  - {}", validators_file.display());
    
    Ok(())
}

/// Create a default configuration file
fn create_default_config(output_file: &PathBuf) -> Result<(), String> {
    let mut params = GenesisParams::default();
    
    // Add some example allocations
    params.allocations.insert("0x7bb62c7d8c215b1ef3e55fc231e77b3ca1b960a3".to_string(), 1_000_000_000_000_000_000);
    params.allocations.insert("0x6a6b09dff11f254c9513096a72b11c5de37e8392".to_string(), 1_000_000_000_000_000_000);
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&params)
        .map_err(|e| format!("Failed to serialize params: {}", e))?;
    
    // Write to file
    fs::write(output_file, json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    println!("Default configuration written to: {}", output_file.display());
    Ok(())
}

/// Calculate Merkle root of transactions
fn compute_transactions_root(transactions: &[Transaction]) -> Hash {
    if transactions.is_empty() {
        return [0u8; 32];
    }
    
    // Get transaction hashes
    let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();
    
    // If we have odd number of leaves, duplicate the last one
    if hashes.len() % 2 == 1 {
        hashes.push(*hashes.last().unwrap());
    }
    
    // Build the Merkle tree
    while hashes.len() > 1 {
        let mut next_level = Vec::new();
        
        for i in (0..hashes.len()).step_by(2) {
            let left = hashes[i];
            let right = hashes[i + 1];
            
            // Combine two hashes with BLAKE3
            let mut hasher = blake3::Hasher::new();
            hasher.update(&left);
            hasher.update(&right);
            let hash = hasher.finalize();
            
            let mut combined = [0u8; 32];
            combined.copy_from_slice(hash.as_bytes());
            
            next_level.push(combined);
        }
        
        hashes = next_level;
    }
    
    // Return the root
    hashes[0]
}

/// Main function for the genesis generator
pub fn main() -> Result<(), String> {
    let args = Args::from_args();
    
    if args.create_config {
        let config_file = args.config.unwrap_or_else(|| PathBuf::from("genesis_config.json"));
        return create_default_config(&config_file);
    }
    
    // Load configuration
    let config_file = match args.config {
        Some(path) => path,
        None => {
            return Err("No configuration file provided. Use --config or --create-config".to_string());
        }
    };
    
    let config_data = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
        
    let params: GenesisParams = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(&args.output)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    // Generate the genesis block
    generate_genesis(&params, &args.output)
}

/// Command to generate a reproducible genesis block
pub fn generate_reproducible_genesis() -> Result<(), String> {
    // Create a fixed timestamp for reproducibility
    let params = GenesisParams {
        timestamp: 1672531200, // 2023-01-01 00:00:00 UTC
        difficulty: 1000,
        nonce: 0,
        allocations: [
            ("0x7bb62c7d8c215b1ef3e55fc231e77b3ca1b960a3".to_string(), 1_000_000_000_000_000_000),
            ("0x6a6b09dff11f254c9513096a72b11c5de37e8392".to_string(), 1_000_000_000_000_000_000),
        ].iter().cloned().collect(),
        validator_count: 3,
        version: 1,
        signature_scheme: "dilithium3".to_string(),
    };
    
    // Use a fixed output directory
    let output_dir = PathBuf::from("genesis");
    
    // Generate genesis files
    generate_genesis(&params, &output_dir)
}
