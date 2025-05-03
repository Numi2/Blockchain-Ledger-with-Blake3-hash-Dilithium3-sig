use crate::contracts::gas::{GasMeter, GasConfig};
use crate::contracts::state::{ContractState, StateAccess};
use crate::types::{Address, Hash};
use wasmer::{imports, Instance, Module, Store, Value, WasmerEnv};
use wasmer_compiler_cranelift::Cranelift;
use thiserror::Error;
use std::sync::Arc;

/// WASM execution errors
#[derive(Error, Debug)]
pub enum WasmError {
    #[error("VM initialization error: {0}")]
    InitializationError(String),
    
    #[error("VM execution error: {0}")]
    ExecutionError(String),
    
    #[error("Out of gas")]
    OutOfGas,
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("Invalid memory access")]
    InvalidMemoryAccess,
    
    #[error("Invalid data format")]
    InvalidDataFormat,
    
    #[error("Invalid contract: {0}")]
    InvalidContract(String),
    
    #[error("Contract not found")]
    ContractNotFound,
    
    #[error("Contract state error: {0}")]
    StateError(String),
}

/// VM execution result
pub type WasmResult<T> = Result<T, WasmError>;

/// Environment for WASM execution
#[derive(Clone)]
pub struct WasmEnv {
    /// Gas meter for execution
    pub gas_meter: Arc<GasMeter>,
    
    /// Contract state
    pub state: Arc<ContractState>,
    
    /// Caller address
    pub caller: Address,
    
    /// Contract address being executed
    pub contract_address: Address,
    
    /// Call value (tokens sent with the call)
    pub value: u64,
    
    /// Block height
    pub block_height: u64,
    
    /// Block timestamp
    pub block_timestamp: u64,
}

impl WasmerEnv for WasmEnv {}

/// WASM VM for smart contract execution
pub struct WasmVM {
    /// Store for WASM modules
    store: Store,
    
    /// Gas configuration
    gas_config: GasConfig,
}

impl WasmVM {
    /// Create a new WASM VM
    pub fn new() -> Self {
        let compiler = Cranelift::default();
        let store = Store::new(&compiler);
        let gas_config = GasConfig::default();
        
        Self {
            store,
            gas_config,
        }
    }
    
    /// Deploy a contract (load WASM bytecode)
    pub fn deploy(&mut self, code: &[u8]) -> WasmResult<Module> {
        // Validate contract code
        if code.is_empty() {
            return Err(WasmError::InvalidContract("Empty contract code".to_string()));
        }
        
        // Load module
        Module::new(&self.store, code)
            .map_err(|e| WasmError::InvalidContract(format!("Failed to load contract: {}", e)))
    }
    
    /// Execute a contract function call
    pub fn execute(
        &mut self,
        module: &Module,
        env: WasmEnv,
        function: &str,
        args: &[Value]
    ) -> WasmResult<Vec<Value>> {
        // Create import objects with environment functions
        let import_object = self.create_import_object(env.clone())?;
        
        // Instantiate the module
        let instance = Instance::new(module, &import_object)
            .map_err(|e| WasmError::InitializationError(format!("Failed to instantiate: {}", e)))?;
        
        // Get the function
        let function = instance.exports.get_function(function)
            .map_err(|_| WasmError::FunctionNotFound(function.to_string()))?;
        
        // Execute the function
        let result = function.call(args)
            .map_err(|e| WasmError::ExecutionError(format!("Execution failed: {}", e)))?;
        
        Ok(result.to_vec())
    }
    
    /// Create import object with host functions
    fn create_import_object(&self, env: WasmEnv) -> WasmResult<wasmer::Imports> {
        let env_clone = env.clone();
        
        // Define imports
        let import_object = imports! {
            "env" => {
                // Get storage value
                "storage_get" => wasmer::Function::new_native_with_env(
                    &self.store,
                    env.clone(),
                    move |env: &WasmEnv, key_ptr: u32, key_len: u32, value_ptr: u32, value_len_ptr: u32| -> i32 {
                        // Charge gas for storage read
                        if let Err(_) = env.gas_meter.charge(self.gas_config.storage_read_base + 
                                                       self.gas_config.storage_read_per_byte * key_len as u64) {
                            return -1; // Out of gas
                        }
                        
                        // Read key from memory
                        // In real implementation, you'd access WASM memory and read the key bytes
                        
                        // For demonstration, we'll just simulate a read
                        1 // Success
                    }
                ),
                
                // Set storage value
                "storage_set" => wasmer::Function::new_native_with_env(
                    &self.store,
                    env.clone(),
                    move |env: &WasmEnv, key_ptr: u32, key_len: u32, value_ptr: u32, value_len: u32| -> i32 {
                        // Charge gas for storage write
                        if let Err(_) = env.gas_meter.charge(self.gas_config.storage_write_base + 
                                               self.gas_config.storage_write_per_byte * (key_len + value_len) as u64) {
                            return -1; // Out of gas
                        }
                        
                        // Read key and value from memory
                        // In real implementation, you'd access WASM memory and read the bytes
                        
                        // For demonstration, we'll just simulate a write
                        1 // Success
                    }
                ),
                
                // Get caller address
                "get_caller" => wasmer::Function::new_native_with_env(
                    &self.store, 
                    env.clone(),
                    move |env: &WasmEnv, ptr: u32| -> i32 {
                        // Charge gas
                        if let Err(_) = env.gas_meter.charge(self.gas_config.external_call_base) {
                            return -1; // Out of gas
                        }
                        
                        // Write caller address to memory
                        // In real implementation, you'd write env.caller to WASM memory at ptr
                        
                        1 // Success
                    }
                ),
                
                // Get block height
                "get_block_height" => wasmer::Function::new_native_with_env(
                    &self.store,
                    env.clone(),
                    move |env: &WasmEnv| -> u64 {
                        // Charge gas
                        if let Err(_) = env.gas_meter.charge(self.gas_config.external_call_base) {
                            return 0; // Out of gas (though this doesn't communicate the error well)
                        }
                        
                        env.block_height
                    }
                ),
                
                // Call another contract
                "call_contract" => wasmer::Function::new_native_with_env(
                    &self.store,
                    env.clone(),
                    move |env: &WasmEnv, addr_ptr: u32, value: u64, 
                           func_ptr: u32, func_len: u32, 
                           args_ptr: u32, args_len: u32,
                           ret_ptr: u32, ret_len_ptr: u32| -> i32 {
                        // Charge gas for contract call
                        if let Err(_) = env.gas_meter.charge(self.gas_config.external_call_base + 
                                                       self.gas_config.external_call_per_byte * args_len as u64) {
                            return -1; // Out of gas
                        }
                        
                        // Read parameters from memory
                        // In real implementation, you'd read contract address, function name, and args
                        
                        // For demonstration, we'll just simulate a contract call
                        1 // Success
                    }
                ),
                
                // Log an event
                "log_event" => wasmer::Function::new_native_with_env(
                    &self.store,
                    env.clone(),
                    move |env: &WasmEnv, topic_ptr: u32, topic_len: u32, data_ptr: u32, data_len: u32| -> i32 {
                        // Charge gas for logging
                        if let Err(_) = env.gas_meter.charge(self.gas_config.log_base + 
                                               self.gas_config.log_per_byte * (topic_len + data_len) as u64) {
                            return -1; // Out of gas
                        }
                        
                        // Read topic and data from memory
                        // In real implementation, you'd access WASM memory and emit a log event
                        
                        1 // Success
                    }
                ),
            }
        };
        
        Ok(import_object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Example WASM contract code that adds two numbers
    const TEST_CONTRACT: &[u8] = include_bytes!("../../tests/contracts/calculator.wasm");
    
    #[test]
    fn test_deploy_contract() {
        let mut vm = WasmVM::new();
        let module = vm.deploy(TEST_CONTRACT).expect("Contract deployment failed");
        
        assert!(module.info().memories.len() > 0);
    }
    
    #[test]
    fn test_execute_contract() {
        let mut vm = WasmVM::new();
        let module = vm.deploy(TEST_CONTRACT).expect("Contract deployment failed");
        
        let gas_meter = Arc::new(GasMeter::new(1_000_000));
        let state = Arc::new(ContractState::new());
        
        let env = WasmEnv {
            gas_meter,
            state,
            caller: [0u8; 20],
            contract_address: [0u8; 20],
            value: 0,
            block_height: 1,
            block_timestamp: 1630000000,
        };
        
        let result = vm.execute(
            &module,
            env,
            "add",
            &[Value::I32(5), Value::I32(7)],
        ).expect("Contract execution failed");
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Value::I32(12));
    }
} 