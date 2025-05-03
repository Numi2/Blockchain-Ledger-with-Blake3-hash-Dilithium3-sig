use crate::contracts::compiler::{CompilerConfig, CompilerError, CompilerResult};
use wasmparser::{Parser, Payload, Validator, WasmFeatures, ValidatingParser, ValidatingParserConfig};
use std::collections::HashSet;
use tracing::{debug, error, info, warn};

/// WebAssembly feature configuration
const ALLOWED_WASM_FEATURES: WasmFeatures = WasmFeatures {
    mutable_global: true,
    saturating_float_to_int: true,
    sign_extension: true,
    reference_types: false,
    multi_value: true,
    bulk_memory: true,
    simd: false,
    relaxed_simd: false,
    threads: false,
    tail_call: false,
    deterministic_only: true,
    multi_memory: false,
    exceptions: false,
    memory64: false,
    extended_const: true,
    component_model: false,
    function_references: false,
    memory_control: false,
    gc: false,
};

/// List of allowed imports
const ALLOWED_IMPORTS: &[&str] = &[
    "env.memory",
    "env.abort",
    "env.gas",
    "env.storage_read",
    "env.storage_write",
    "env.storage_remove",
    "env.storage_has_key",
    "env.result_size",
    "env.write_result",
    "env.read_input",
    "env.input_size",
    "env.block_height",
    "env.block_timestamp",
    "env.current_account_id",
    "env.caller_account_id",
    "env.emit_event",
    "env.log_message",
    "env.assert",
];

/// Maximum allowed table elements
const MAX_TABLE_ELEMENTS: u32 = 1000;

/// Maximum allowed function count
const MAX_FUNCTION_COUNT: u32 = 1000;

/// Maximum allowed export count
const MAX_EXPORT_COUNT: u32 = 100;

/// Maximum allowed global variables
const MAX_GLOBAL_COUNT: u32 = 100;

/// Verify a WebAssembly binary for security and compatibility
pub fn verify_wasm(wasm_binary: &[u8], config: &CompilerConfig) -> CompilerResult<()> {
    // Validate basic WebAssembly structure
    validate_wasm_structure(wasm_binary)?;
    
    // Check for dangerous imports
    validate_imports(wasm_binary)?;
    
    // Check for size constraints
    validate_size_constraints(wasm_binary, config)?;
    
    // Validate exports
    validate_exports(wasm_binary)?;
    
    // Verify memory limits
    validate_memory_limits(wasm_binary, config)?;
    
    // Check for floating point operations if needed
    validate_no_floating_point(wasm_binary)?;
    
    // Check for start section (disallowed)
    validate_no_start_section(wasm_binary)?;
    
    // All validations passed
    Ok(())
}

/// Validate basic WebAssembly structure
fn validate_wasm_structure(wasm_binary: &[u8]) -> CompilerResult<()> {
    // Create validator with allowed features
    let config = ValidatingParserConfig::new()
        .with_features(ALLOWED_WASM_FEATURES);
    
    let mut validator = ValidatingParser::new(wasm_binary, config);
    
    // Parse and validate the module
    let mut error = None;
    for payload in validator {
        if let Err(err) = payload {
            error = Some(err);
            break;
        }
    }
    
    // Return error if validation failed
    if let Some(err) = error {
        return Err(CompilerError::Verification(
            format!("Invalid WebAssembly module: {}", err)
        ));
    }
    
    Ok(())
}

/// Validate WebAssembly imports
fn validate_imports(wasm_binary: &[u8]) -> CompilerResult<()> {
    let mut parser = Parser::new(0);
    let mut imports = Vec::new();
    
    // Parse the module to collect imports
    for payload in parser.parse_all(wasm_binary) {
        let payload = payload.map_err(|e| {
            CompilerError::Verification(format!("Failed to parse WASM: {}", e))
        })?;
        
        // Check for imports
        if let Payload::ImportSection(section) {
            imports.extend(section.collect::<Result<Vec<_>, _>>().map_err(|e| {
                CompilerError::Verification(format!("Failed to parse import section: {}", e))
            })?);
        }
    }
    
    // Create a set of allowed imports for faster lookup
    let allowed_imports: HashSet<&str> = ALLOWED_IMPORTS.iter().copied().collect();
    
    // Check each import against the allowed list
    for import in imports {
        let import_name = format!("{}.{}", import.module, import.name);
        if !allowed_imports.contains(import_name.as_str()) {
            return Err(CompilerError::Verification(
                format!("Disallowed import: {}", import_name)
            ));
        }
    }
    
    Ok(())
}

/// Validate size constraints
fn validate_size_constraints(wasm_binary: &[u8], config: &CompilerConfig) -> CompilerResult<()> {
    let mut parser = Parser::new(0);
    let mut function_count = 0;
    let mut export_count = 0;
    let mut global_count = 0;
    let mut table_element_count = 0;
    
    // Parse the module to collect constraints
    for payload in parser.parse_all(wasm_binary) {
        let payload = payload.map_err(|e| {
            CompilerError::Verification(format!("Failed to parse WASM: {}", e))
        })?;
        
        match payload {
            Payload::FunctionSection(section) => {
                function_count += section.count();
            },
            Payload::ExportSection(section) => {
                export_count = section.count();
            },
            Payload::GlobalSection(section) => {
                global_count = section.count();
            },
            Payload::ElementSection(section) => {
                for element in section {
                    let element = element.map_err(|e| {
                        CompilerError::Verification(format!("Failed to parse element section: {}", e))
                    })?;
                    table_element_count += element.items.count();
                }
            },
            _ => {}
        }
    }
    
    // Check constraints
    if function_count > MAX_FUNCTION_COUNT {
        return Err(CompilerError::Verification(
            format!("Too many functions: {} (max: {})", function_count, MAX_FUNCTION_COUNT)
        ));
    }
    
    if export_count > MAX_EXPORT_COUNT {
        return Err(CompilerError::Verification(
            format!("Too many exports: {} (max: {})", export_count, MAX_EXPORT_COUNT)
        ));
    }
    
    if global_count > MAX_GLOBAL_COUNT {
        return Err(CompilerError::Verification(
            format!("Too many globals: {} (max: {})", global_count, MAX_GLOBAL_COUNT)
        ));
    }
    
    if table_element_count > MAX_TABLE_ELEMENTS {
        return Err(CompilerError::Verification(
            format!("Too many table elements: {} (max: {})", table_element_count, MAX_TABLE_ELEMENTS)
        ));
    }
    
    Ok(())
}

/// Validate WebAssembly exports
fn validate_exports(wasm_binary: &[u8]) -> CompilerResult<()> {
    let mut parser = Parser::new(0);
    let mut found_memory_export = false;
    
    // Parse the module to validate exports
    for payload in parser.parse_all(wasm_binary) {
        let payload = payload.map_err(|e| {
            CompilerError::Verification(format!("Failed to parse WASM: {}", e))
        })?;
        
        if let Payload::ExportSection(section) {
            let exports = section.collect::<Result<Vec<_>, _>>().map_err(|e| {
                CompilerError::Verification(format!("Failed to parse export section: {}", e))
            })?;
            
            // Check for required exports
            for export in &exports {
                if export.name == "memory" {
                    found_memory_export = true;
                }
            }
        }
    }
    
    // Memory export is optional - we can import it from the environment
    
    Ok(())
}

/// Validate memory limits
fn validate_memory_limits(wasm_binary: &[u8], config: &CompilerConfig) -> CompilerResult<()> {
    let mut parser = Parser::new(0);
    
    // Parse the module to validate memory
    for payload in parser.parse_all(wasm_binary) {
        let payload = payload.map_err(|e| {
            CompilerError::Verification(format!("Failed to parse WASM: {}", e))
        })?;
        
        if let Payload::MemorySection(section) {
            for memory in section {
                let memory = memory.map_err(|e| {
                    CompilerError::Verification(format!("Failed to parse memory section: {}", e))
                })?;
                
                // Check initial size
                if memory.initial > config.max_memory_pages {
                    return Err(CompilerError::Verification(
                        format!("Initial memory size too large: {} pages (max: {} pages)",
                            memory.initial, config.max_memory_pages)
                    ));
                }
                
                // Check maximum size if specified
                if let Some(max) = memory.maximum {
                    if max > config.max_memory_pages {
                        return Err(CompilerError::Verification(
                            format!("Maximum memory size too large: {} pages (max: {} pages)",
                                max, config.max_memory_pages)
                        ));
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Validate that there are no floating point operations
fn validate_no_floating_point(wasm_binary: &[u8]) -> CompilerResult<()> {
    // This is a more complex validation that requires analyzing the code section
    // For now, we'll implement a simple check for floating point opcodes
    let mut parser = Parser::new(0);
    
    for payload in parser.parse_all(wasm_binary) {
        let payload = payload.map_err(|e| {
            CompilerError::Verification(format!("Failed to parse WASM: {}", e))
        })?;
        
        if let Payload::CodeSectionEntry(body) {
            let ops = body.get_operators_reader();
            
            for op in ops {
                let op = op.map_err(|e| {
                    CompilerError::Verification(format!("Failed to parse code section: {}", e))
                })?;
                
                // Check for floating point operations
                match op {
                    wasmparser::Operator::F32Abs |
                    wasmparser::Operator::F32Add |
                    wasmparser::Operator::F32Ceil |
                    wasmparser::Operator::F32Const { .. } |
                    wasmparser::Operator::F32ConvertI32S |
                    wasmparser::Operator::F32ConvertI32U |
                    wasmparser::Operator::F32ConvertI64S |
                    wasmparser::Operator::F32ConvertI64U |
                    wasmparser::Operator::F32Copysign |
                    wasmparser::Operator::F32Div |
                    wasmparser::Operator::F32Eq |
                    wasmparser::Operator::F32Floor |
                    wasmparser::Operator::F32Ge |
                    wasmparser::Operator::F32Gt |
                    wasmparser::Operator::F32Le |
                    wasmparser::Operator::F32Lt |
                    wasmparser::Operator::F32Max |
                    wasmparser::Operator::F32Min |
                    wasmparser::Operator::F32Mul |
                    wasmparser::Operator::F32Ne |
                    wasmparser::Operator::F32Nearest |
                    wasmparser::Operator::F32Neg |
                    wasmparser::Operator::F32Reinterpret |
                    wasmparser::Operator::F32Sqrt |
                    wasmparser::Operator::F32Sub |
                    wasmparser::Operator::F32Trunc |
                    wasmparser::Operator::F64Abs |
                    wasmparser::Operator::F64Add |
                    wasmparser::Operator::F64Ceil |
                    wasmparser::Operator::F64Const { .. } |
                    wasmparser::Operator::F64ConvertI32S |
                    wasmparser::Operator::F64ConvertI32U |
                    wasmparser::Operator::F64ConvertI64S |
                    wasmparser::Operator::F64ConvertI64U |
                    wasmparser::Operator::F64Copysign |
                    wasmparser::Operator::F64Div |
                    wasmparser::Operator::F64Eq |
                    wasmparser::Operator::F64Floor |
                    wasmparser::Operator::F64Ge |
                    wasmparser::Operator::F64Gt |
                    wasmparser::Operator::F64Le |
                    wasmparser::Operator::F64Lt |
                    wasmparser::Operator::F64Max |
                    wasmparser::Operator::F64Min |
                    wasmparser::Operator::F64Mul |
                    wasmparser::Operator::F64Ne |
                    wasmparser::Operator::F64Nearest |
                    wasmparser::Operator::F64Neg |
                    wasmparser::Operator::F64Reinterpret |
                    wasmparser::Operator::F64Sqrt |
                    wasmparser::Operator::F64Sub |
                    wasmparser::Operator::F64Trunc => {
                        return Err(CompilerError::Verification(
                            "Floating point operations are not allowed in contracts".to_string()
                        ));
                    },
                    _ => {}
                }
            }
        }
    }
    
    Ok(())
}

/// Validate that there is no start section
fn validate_no_start_section(wasm_binary: &[u8]) -> CompilerResult<()> {
    let mut parser = Parser::new(0);
    
    for payload in parser.parse_all(wasm_binary) {
        let payload = payload.map_err(|e| {
            CompilerError::Verification(format!("Failed to parse WASM: {}", e))
        })?;
        
        if let Payload::StartSection { .. } {
            return Err(CompilerError::Verification(
                "Start section is not allowed in contracts".to_string()
            ));
        }
    }
    
    Ok(())
}

/// Perform security checks on WebAssembly binary
pub fn security_scan(wasm_binary: &[u8]) -> CompilerResult<Vec<String>> {
    let mut warnings = Vec::new();
    
    // Check for deterministic behavior
    if let Err(e) = check_deterministic_behavior(wasm_binary) {
        warnings.push(format!("Non-deterministic behavior detected: {}", e));
    }
    
    // Check for unbounded loops
    if let Err(e) = check_unbounded_loops(wasm_binary) {
        warnings.push(format!("Potentially unbounded loops detected: {}", e));
    }
    
    // Check for expensive operations
    if let Err(e) = check_expensive_operations(wasm_binary) {
        warnings.push(format!("Expensive operations detected: {}", e));
    }
    
    // Check for reentrancy
    if let Err(e) = check_reentrancy(wasm_binary) {
        warnings.push(format!("Potential reentrancy vulnerability: {}", e));
    }
    
    Ok(warnings)
}

/// Check for deterministic behavior
fn check_deterministic_behavior(wasm_binary: &[u8]) -> CompilerResult<()> {
    // This is a placeholder for more complex analysis
    // A real implementation would check for things like:
    // - Usage of time-based functions
    // - Usage of random number generation
    // - Usage of floating point operations (already checked in validate_no_floating_point)
    
    Ok(())
}

/// Check for unbounded loops
fn check_unbounded_loops(wasm_binary: &[u8]) -> CompilerResult<()> {
    // This is a placeholder for more complex analysis
    // A real implementation would analyze the control flow graph to detect:
    // - Loops without a clear exit condition
    // - Recursive functions without a termination condition
    
    Ok(())
}

/// Check for expensive operations
fn check_expensive_operations(wasm_binary: &[u8]) -> CompilerResult<()> {
    // This is a placeholder for more complex analysis
    // A real implementation would look for:
    // - O(n²) or worse algorithms
    // - Excessive memory allocation
    // - Complex computations
    
    Ok(())
}

/// Check for reentrancy vulnerabilities
fn check_reentrancy(wasm_binary: &[u8]) -> CompilerResult<()> {
    // This is a placeholder for more complex analysis
    // A real implementation would look for:
    // - State changes after external calls
    // - Multiple external calls that could be reentrant
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Sample valid WebAssembly module for testing
    const VALID_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // Magic + version
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00,             // Type section
        0x03, 0x02, 0x01, 0x00,                         // Function section
        0x07, 0x07, 0x01, 0x03, 0x69, 0x6e, 0x69, 0x74, 0x00, 0x00, // Export section
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,             // Code section
    ];
    
    #[test]
    fn test_verify_wasm_structure() {
        let config = CompilerConfig::default();
        let result = validate_wasm_structure(VALID_WASM);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_verify_wasm() {
        let config = CompilerConfig::default();
        let result = verify_wasm(VALID_WASM, &config);
        assert!(result.is_ok());
    }
} 