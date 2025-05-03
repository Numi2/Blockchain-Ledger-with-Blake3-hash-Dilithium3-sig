use crate::types::{Address, Hash, Transaction};
use crate::wallet::{AccountManager, SignedTransaction};
use crate::core::state::WorldState;
use crate::core::mempool::MemPool;
use crate::consensus::types::Consensus;
use crate::p2p::protocol::NodeStatus;

use jsonrpc_http_server::{HttpServer, ServerBuilder};
use jsonrpc_core::{IoHandler, Error, Result, Params, Value};
use jsonrpc_derive::rpc;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::net::SocketAddr;

/// RPC configuration
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// RPC server address
    pub address: SocketAddr,
    
    /// Enable debug endpoints
    pub enable_debug: bool,
    
    /// Maximum connections
    pub max_connections: usize,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:8545".parse().unwrap(),
            enable_debug: false,
            max_connections: 100,
        }
    }
}

/// Transaction submission response
#[derive(Debug, Serialize, Deserialize)]
pub struct TxSubmitResponse {
    /// Transaction hash
    pub tx_hash: String,
    
    /// Transaction inclusion status
    pub status: String,
}

/// Block information
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block hash
    pub hash: String,
    
    /// Block height
    pub height: u64,
    
    /// Block timestamp
    pub timestamp: u64,
    
    /// Number of transactions
    pub tx_count: usize,
    
    /// Block producer
    pub producer: String,
    
    /// Block size in bytes
    pub size: usize,
}

/// Account information
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account address
    pub address: String,
    
    /// Account balance
    pub balance: String,
    
    /// Account nonce
    pub nonce: u64,
    
    /// Whether this is a contract account
    pub is_contract: bool,
}

/// Transaction information
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction hash
    pub hash: String,
    
    /// Block hash (if included in a block)
    pub block_hash: Option<String>,
    
    /// Block height (if included in a block)
    pub block_height: Option<u64>,
    
    /// Transaction index in block
    pub tx_index: Option<usize>,
    
    /// Sender address
    pub from: String,
    
    /// Recipient address (if any)
    pub to: Option<String>,
    
    /// Transaction value
    pub value: String,
    
    /// Gas price
    pub gas_price: String,
    
    /// Gas limit
    pub gas_limit: String,
    
    /// Transaction nonce
    pub nonce: u64,
    
    /// Transaction data
    pub data: String,
    
    /// Transaction timestamp
    pub timestamp: Option<u64>,
    
    /// Transaction status
    pub status: String,
}

/// API exposed by the RPC server
#[rpc]
pub trait WorldLedgerRpc {
    /// Get node status
    #[rpc(name = "status")]
    fn status(&self) -> Result<NodeStatus>;
    
    /// Get block by hash or height
    #[rpc(name = "getBlock")]
    fn get_block(&self, hash_or_height: Value) -> Result<Option<BlockInfo>>;
    
    /// Get account information
    #[rpc(name = "getAccount")]
    fn get_account(&self, address: String) -> Result<Option<AccountInfo>>;
    
    /// Get transaction by hash
    #[rpc(name = "getTransaction")]
    fn get_transaction(&self, hash: String) -> Result<Option<TransactionInfo>>;
    
    /// Submit a raw transaction
    #[rpc(name = "sendRawTransaction")]
    fn send_raw_transaction(&self, tx_hex: String) -> Result<TxSubmitResponse>;
    
    /// Submit a transaction
    #[rpc(name = "sendTransaction")]
    fn send_transaction(&self, tx: Value) -> Result<TxSubmitResponse>;
    
    /// Get transaction count for address (nonce)
    #[rpc(name = "getTransactionCount")]
    fn get_transaction_count(&self, address: String) -> Result<u64>;
    
    /// Get balance for address
    #[rpc(name = "getBalance")]
    fn get_balance(&self, address: String) -> Result<String>;
    
    /// Estimate gas for transaction
    #[rpc(name = "estimateGas")]
    fn estimate_gas(&self, tx: Value) -> Result<String>;
    
    /// Get current gas price
    #[rpc(name = "gasPrice")]
    fn gas_price(&self) -> Result<String>;
    
    /// Create and sign a transaction (debug only)
    #[rpc(name = "debug_createTransaction")]
    fn create_transaction(&self, from: String, to: String, value: String) -> Result<TxSubmitResponse>;
}

/// RPC server implementation
pub struct RpcServer<C: Consensus> {
    /// Node status provider
    status_provider: Arc<dyn Fn() -> NodeStatus + Send + Sync>,
    
    /// World state
    world_state: Arc<RwLock<WorldState>>,
    
    /// Memory pool
    mempool: Arc<RwLock<MemPool>>,
    
    /// Account manager
    account_manager: Option<Arc<RwLock<AccountManager>>>,
    
    /// Consensus mechanism
    consensus: Arc<RwLock<C>>,
    
    /// RPC configuration
    config: RpcConfig,
}

impl<C: Consensus + 'static> RpcServer<C> {
    /// Create a new RPC server
    pub fn new(
        status_provider: Arc<dyn Fn() -> NodeStatus + Send + Sync>,
        world_state: Arc<RwLock<WorldState>>,
        mempool: Arc<RwLock<MemPool>>,
        consensus: Arc<RwLock<C>>,
        config: RpcConfig,
    ) -> Self {
        Self {
            status_provider,
            world_state,
            mempool,
            account_manager: None,
            consensus,
            config,
        }
    }
    
    /// Set the account manager (optional)
    pub fn with_account_manager(mut self, account_manager: Arc<RwLock<AccountManager>>) -> Self {
        self.account_manager = Some(account_manager);
        self
    }
    
    /// Start the RPC server
    pub fn start(&self) -> Result<HttpServer> {
        let mut io = IoHandler::default();
        
        // Clone necessary components for the server
        let status_provider = self.status_provider.clone();
        let world_state = self.world_state.clone();
        let mempool = self.mempool.clone();
        let account_manager = self.account_manager.clone();
        let config = self.config.clone();
        
        // Implement RPC methods
        io.extend_with(
            Self::to_delegate(RpcServerImpl {
                status_provider,
                world_state,
                mempool,
                account_manager,
                enable_debug: config.enable_debug,
            })
        );
        
        // Create server
        let server = ServerBuilder::new(io)
            .threads(4)
            .max_request_body_size(10 * 1024 * 1024) // 10 MB
            .start_http(&config.address)
            .map_err(|e| Error::invalid_request_with_details(e, "Failed to start RPC server"))?;
            
        println!("RPC server running on {}", config.address);
        
        Ok(server)
    }
}

/// RPC server implementation
struct RpcServerImpl {
    /// Node status provider
    status_provider: Arc<dyn Fn() -> NodeStatus + Send + Sync>,
    
    /// World state
    world_state: Arc<RwLock<WorldState>>,
    
    /// Memory pool
    mempool: Arc<RwLock<MemPool>>,
    
    /// Account manager (optional)
    account_manager: Option<Arc<RwLock<AccountManager>>>,
    
    /// Enable debug endpoints
    enable_debug: bool,
}

impl WorldLedgerRpc for RpcServerImpl {
    fn status(&self) -> Result<NodeStatus> {
        Ok((self.status_provider)())
    }
    
    fn get_block(&self, hash_or_height: Value) -> Result<Option<BlockInfo>> {
        // This is a simplified implementation
        // In a real blockchain, you would look up blocks in storage
        
        // For demonstration, return a dummy block
        Ok(Some(BlockInfo {
            hash: "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            height: 1,
            timestamp: 1630000000,
            tx_count: 1,
            producer: "0x0123456789abcdef0123456789abcdef01234567".to_string(),
            size: 1024,
        }))
    }
    
    fn get_account(&self, address_str: String) -> Result<Option<AccountInfo>> {
        // Parse address
        let address_bytes = hex::decode(address_str.strip_prefix("0x").unwrap_or(&address_str))
            .map_err(|_| Error::invalid_params("Invalid address format"))?;
            
        let mut address = [0u8; 20];
        if address_bytes.len() != 20 {
            return Err(Error::invalid_params("Address must be 20 bytes"));
        }
        address.copy_from_slice(&address_bytes);
        
        // Get account from world state
        let world_state = self.world_state.read()
            .map_err(|_| Error::internal_error())?;
            
        let account = world_state.get_account(&address);
        
        match account {
            Some(account) => {
                Ok(Some(AccountInfo {
                    address: format!("0x{}", hex::encode(address)),
                    balance: account.balance.to_string(),
                    nonce: account.nonce,
                    is_contract: !account.code.is_empty(),
                }))
            },
            None => Ok(None),
        }
    }
    
    fn get_transaction(&self, hash_str: String) -> Result<Option<TransactionInfo>> {
        // Parse hash
        let hash_bytes = hex::decode(hash_str.strip_prefix("0x").unwrap_or(&hash_str))
            .map_err(|_| Error::invalid_params("Invalid hash format"))?;
            
        let mut hash = [0u8; 32];
        if hash_bytes.len() != 32 {
            return Err(Error::invalid_params("Hash must be 32 bytes"));
        }
        hash.copy_from_slice(&hash_bytes);
        
        // Check mempool for the transaction
        let mempool = self.mempool.read()
            .map_err(|_| Error::internal_error())?;
            
        if let Some(tx) = mempool.get_transaction(&hash) {
            return Ok(Some(TransactionInfo {
                hash: format!("0x{}", hex::encode(hash)),
                block_hash: None,
                block_height: None,
                tx_index: None,
                from: format!("0x{}", hex::encode(tx.from)),
                to: tx.to.map(|addr| format!("0x{}", hex::encode(addr))),
                value: tx.value.to_string(),
                gas_price: tx.gas_price.to_string(),
                gas_limit: tx.gas_limit.to_string(),
                nonce: tx.nonce,
                data: format!("0x{}", hex::encode(&tx.data)),
                timestamp: None,
                status: "pending".to_string(),
            }));
        }
        
        // If not in mempool, check chain (not implemented)
        Ok(None)
    }
    
    fn send_raw_transaction(&self, tx_hex: String) -> Result<TxSubmitResponse> {
        // Parse transaction bytes
        let tx_bytes = hex::decode(tx_hex.strip_prefix("0x").unwrap_or(&tx_hex))
            .map_err(|_| Error::invalid_params("Invalid transaction data"))?;
            
        // Deserialize transaction
        let signed_tx: SignedTransaction = bincode::deserialize(&tx_bytes)
            .map_err(|_| Error::invalid_params("Invalid transaction format"))?;
            
        // Extract transaction and hash
        let tx = signed_tx.transaction;
        let tx_hash = tx.hash();
        
        // Add to mempool
        let mut mempool = self.mempool.write()
            .map_err(|_| Error::internal_error())?;
            
        mempool.add_transaction(tx)
            .map_err(|e| Error::invalid_params(format!("Transaction rejected: {}", e)))?;
            
        Ok(TxSubmitResponse {
            tx_hash: format!("0x{}", hex::encode(tx_hash)),
            status: "pending".to_string(),
        })
    }
    
    fn send_transaction(&self, tx_params: Value) -> Result<TxSubmitResponse> {
        // Require account manager for this method
        let account_manager = match &self.account_manager {
            Some(am) => am.read().map_err(|_| Error::internal_error())?,
            None => return Err(Error::method_not_found()),
        };
        
        // Parse transaction parameters
        let from = tx_params.get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid_params("Missing 'from' parameter"))?;
            
        let to = tx_params.get("to")
            .and_then(|v| v.as_str());
            
        let value = tx_params.get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
            
        let gas = tx_params.get("gas")
            .and_then(|v| v.as_str())
            .unwrap_or("21000");
            
        let gas_price = tx_params.get("gasPrice")
            .and_then(|v| v.as_str())
            .unwrap_or("1000000000");
            
        let data = tx_params.get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("0x");
            
        // Parse addresses and values
        let from_bytes = hex::decode(from.strip_prefix("0x").unwrap_or(from))
            .map_err(|_| Error::invalid_params("Invalid from address"))?;
            
        let mut from_addr = [0u8; 20];
        if from_bytes.len() != 20 {
            return Err(Error::invalid_params("From address must be 20 bytes"));
        }
        from_addr.copy_from_slice(&from_bytes);
        
        let to_addr = if let Some(to) = to {
            let to_bytes = hex::decode(to.strip_prefix("0x").unwrap_or(to))
                .map_err(|_| Error::invalid_params("Invalid to address"))?;
                
            let mut addr = [0u8; 20];
            if to_bytes.len() != 20 {
                return Err(Error::invalid_params("To address must be 20 bytes"));
            }
            addr.copy_from_slice(&to_bytes);
            Some(addr)
        } else {
            None
        };
        
        let value_u64 = value.parse::<u64>()
            .map_err(|_| Error::invalid_params("Invalid value"))?;
            
        let gas_u64 = gas.parse::<u64>()
            .map_err(|_| Error::invalid_params("Invalid gas"))?;
            
        let gas_price_u64 = gas_price.parse::<u64>()
            .map_err(|_| Error::invalid_params("Invalid gasPrice"))?;
            
        let data_bytes = hex::decode(data.strip_prefix("0x").unwrap_or(data))
            .map_err(|_| Error::invalid_params("Invalid data"))?;
            
        // Get account nonce
        let world_state = self.world_state.read()
            .map_err(|_| Error::internal_error())?;
            
        let nonce = match world_state.get_account(&from_addr) {
            Some(account) => account.nonce,
            None => 0,
        };
        
        // Create transaction
        let tx = Transaction {
            from: from_addr,
            to: to_addr,
            value: value_u64,
            gas_limit: gas_u64,
            gas_price: gas_price_u64,
            nonce,
            data: data_bytes,
        };
        
        // Add to mempool
        let tx_hash = tx.hash();
        
        let mut mempool = self.mempool.write()
            .map_err(|_| Error::internal_error())?;
            
        mempool.add_transaction(tx)
            .map_err(|e| Error::invalid_params(format!("Transaction rejected: {}", e)))?;
            
        Ok(TxSubmitResponse {
            tx_hash: format!("0x{}", hex::encode(tx_hash)),
            status: "pending".to_string(),
        })
    }
    
    fn get_transaction_count(&self, address_str: String) -> Result<u64> {
        // Parse address
        let address_bytes = hex::decode(address_str.strip_prefix("0x").unwrap_or(&address_str))
            .map_err(|_| Error::invalid_params("Invalid address format"))?;
            
        let mut address = [0u8; 20];
        if address_bytes.len() != 20 {
            return Err(Error::invalid_params("Address must be 20 bytes"));
        }
        address.copy_from_slice(&address_bytes);
        
        // Get account from world state
        let world_state = self.world_state.read()
            .map_err(|_| Error::internal_error())?;
            
        let nonce = match world_state.get_account(&address) {
            Some(account) => account.nonce,
            None => 0,
        };
        
        Ok(nonce)
    }
    
    fn get_balance(&self, address_str: String) -> Result<String> {
        // Parse address
        let address_bytes = hex::decode(address_str.strip_prefix("0x").unwrap_or(&address_str))
            .map_err(|_| Error::invalid_params("Invalid address format"))?;
            
        let mut address = [0u8; 20];
        if address_bytes.len() != 20 {
            return Err(Error::invalid_params("Address must be 20 bytes"));
        }
        address.copy_from_slice(&address_bytes);
        
        // Get account from world state
        let world_state = self.world_state.read()
            .map_err(|_| Error::internal_error())?;
            
        let balance = match world_state.get_account(&address) {
            Some(account) => account.balance,
            None => 0,
        };
        
        Ok(balance.to_string())
    }
    
    fn estimate_gas(&self, _tx: Value) -> Result<String> {
        // Simple implementation - just return default gas limit
        Ok("21000".to_string())
    }
    
    fn gas_price(&self) -> Result<String> {
        // Simple implementation - just return default gas price
        Ok("1000000000".to_string())
    }
    
    fn create_transaction(&self, from: String, to: String, value: String) -> Result<TxSubmitResponse> {
        // Check if debug is enabled
        if !self.enable_debug {
            return Err(Error::method_not_found());
        }
        
        // Require account manager for this method
        let account_manager = match &self.account_manager {
            Some(am) => am.read().map_err(|_| Error::internal_error())?,
            None => return Err(Error::method_not_found()),
        };
        
        // Parse addresses and values
        let from_bytes = hex::decode(from.strip_prefix("0x").unwrap_or(&from))
            .map_err(|_| Error::invalid_params("Invalid from address"))?;
            
        let mut from_addr = [0u8; 20];
        if from_bytes.len() != 20 {
            return Err(Error::invalid_params("From address must be 20 bytes"));
        }
        from_addr.copy_from_slice(&from_bytes);
        
        let to_bytes = hex::decode(to.strip_prefix("0x").unwrap_or(&to))
            .map_err(|_| Error::invalid_params("Invalid to address"))?;
            
        let mut to_addr = [0u8; 20];
        if to_bytes.len() != 20 {
            return Err(Error::invalid_params("To address must be 20 bytes"));
        }
        to_addr.copy_from_slice(&to_bytes);
        
        let value_u64 = value.parse::<u64>()
            .map_err(|_| Error::invalid_params("Invalid value"))?;
            
        // Get account nonce
        let world_state = self.world_state.read()
            .map_err(|_| Error::internal_error())?;
            
        let nonce = match world_state.get_account(&from_addr) {
            Some(account) => account.nonce,
            None => 0,
        };
        
        // Create transaction
        let tx = Transaction {
            from: from_addr,
            to: Some(to_addr),
            value: value_u64,
            gas_limit: 21000,
            gas_price: 1000000000,
            nonce,
            data: Vec::new(),
        };
        
        // Add to mempool
        let tx_hash = tx.hash();
        
        let mut mempool = self.mempool.write()
            .map_err(|_| Error::internal_error())?;
            
        mempool.add_transaction(tx)
            .map_err(|e| Error::invalid_params(format!("Transaction rejected: {}", e)))?;
            
        Ok(TxSubmitResponse {
            tx_hash: format!("0x{}", hex::encode(tx_hash)),
            status: "pending".to_string(),
        })
    }
} 