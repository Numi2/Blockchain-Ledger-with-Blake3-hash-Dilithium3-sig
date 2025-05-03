use super::*;
use wl::contracts::vm::{WasmVM, WasmEnv};
use wl::contracts::gas::GasMeter;
use wl::contracts::state::ContractState;
use wl::contracts::abi::ContractAbi;
use wl::contracts::runtime::{ContractRegistry, ContractContext};
use std::sync::Arc;
use std::time::Duration;
use wasmer::Value;

/// Simple test WASM contract (adder function)
const TEST_CONTRACT: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01,
    0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, 0x0a, 0x09,
    0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b
];

/// Simple ABI for the test contract
const TEST_ABI: &str = r#"[
    {
        "type": "function",
        "name": "add",
        "inputs": [
            {"name": "a", "type": "int32"},
            {"name": "b", "type": "int32"}
        ],
        "outputs": [
            {"name": "result", "type": "int32"}
        ],
        "stateMutability": "pure"
    }
]"#;

/// Test basic WASM VM functionality
#[test]
fn test_wasm_vm() {
    let config = setup();
    
    // Create VM
    let mut vm = WasmVM::new();
    
    // Deploy contract
    let module = vm.deploy(TEST_CONTRACT).expect("Failed to deploy contract");
    
    // Create execution environment
    let gas_meter = Arc::new(GasMeter::new(1_000_000));
    let state = Arc::new(ContractState::new());
    
    let env = WasmEnv {
        gas_meter: gas_meter.clone(),
        state: state.clone(),
        caller: [0u8; 20],
        contract_address: [0u8; 20],
        value: 0,
        block_height: 1,
        block_timestamp: 1630000000,
    };
    
    // Execute contract
    let result = vm.execute(
        &module,
        env,
        "add",
        &[Value::I32(5), Value::I32(7)],
    ).expect("Contract execution failed");
    
    // Verify result
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], Value::I32(12));
    
    teardown(config);
}

/// Test contract registry functionality
#[test]
fn test_contract_registry() {
    let config = setup();
    
    // Create contract registry
    let registry = ContractRegistry::new(config.db.clone());
    
    // Create transaction hash for context
    let tx_hash = [1u8; 32];
    
    // Create execution context
    let context = ContractContext {
        block_height: 1,
        block_timestamp: 1630000000,
        tx_hash,
    };
    
    // Deploy contract
    let deploy_result = registry.deploy_contract(
        TEST_CONTRACT.to_vec(),
        TEST_ABI,
        Vec::new(), // No constructor args
        [0u8; 20],  // Sender
        0,          // No value
        1_000_000,  // Gas limit
        &context,
    ).expect("Contract deployment failed");
    
    // Verify deployment
    let contract_address = deploy_result.contract_address;
    
    // Call contract
    use serde_json::json;
    let call_result = registry.call_contract(
        contract_address,
        "add",
        vec![json!(5), json!(7)],
        [0u8; 20],  // Sender
        0,          // No value
        1_000_000,  // Gas limit
        &context,
    ).expect("Contract call failed");
    
    // Gas should have been used
    assert!(call_result.gas_used > 0);
    
    teardown(config);
}

/// Test contract ABI encoding/decoding
#[test]
fn test_contract_abi() {
    // Parse ABI
    let abi = ContractAbi::from_json(TEST_ABI).expect("Failed to parse ABI");
    
    // Check ABI contents
    assert_eq!(abi.functions.len(), 1);
    assert_eq!(abi.functions[0].name, "add");
    assert_eq!(abi.functions[0].inputs.len(), 2);
    assert_eq!(abi.functions[0].outputs.len(), 1);
    
    // Get function selector
    let selector = abi.function_selector("add").expect("Failed to get function selector");
    assert_eq!(selector.len(), 4);
    
    // Encode function call
    use serde_json::json;
    let encoded = abi.encode_function_call("add", &[json!(5), json!(7)])
        .expect("Failed to encode function call");
    
    // Selector should be at the start
    assert_eq!(encoded[0..4], selector);
}

/// Test gas metering
#[test]
fn test_gas_metering() {
    // Create gas meter with limit
    let gas_meter = GasMeter::new(10_000);
    
    // Charge some gas
    assert!(gas_meter.charge(5_000).is_ok());
    assert_eq!(gas_meter.gas_used(), 5_000);
    assert_eq!(gas_meter.gas_remaining(), 5_000);
    
    // Charge more gas
    assert!(gas_meter.charge(4_000).is_ok());
    assert_eq!(gas_meter.gas_used(), 9_000);
    assert_eq!(gas_meter.gas_remaining(), 1_000);
    
    // Charging too much should fail
    assert!(gas_meter.charge(2_000).is_err());
    
    // But we can still charge small amounts
    assert!(gas_meter.charge(500).is_ok());
    assert_eq!(gas_meter.gas_used(), 9_500);
    assert_eq!(gas_meter.gas_remaining(), 500);
} 