use crate::contracts::compiler::{CompilerConfig, CompilerError, CompilerResult};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::thread;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

/// Rust toolchain version
const RUST_TOOLCHAIN: &str = "nightly";

/// Cargo.toml template for smart contracts
const CARGO_TOML_TEMPLATE: &str = r#"
[package]
name = "{{package_name}}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Minimal dependency set for contracts
world-ledger-contract-sdk = { version = "0.1", default-features = false }

[profile.release]
opt-level = 'z'     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Reduce parallel code generation units to increase optimization
panic = 'abort'     # Abort on panic
strip = true        # Strip symbols from binary
overflow-checks = true  # Check for overflows

[features]
default = ["std"]
std = ["world-ledger-contract-sdk/std"]
"#;

/// Rust source template with contract basics
const SOURCE_TEMPLATE: &str = r#"
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_imports)]

use world_ledger_contract_sdk::{
    contract_api::*,
    contract_sdk::{
        alloc::string::String,
        error::Error,
        gas,
        logging,
        storage,
    },
};

// Storage keys
const COUNTER_KEY: &[u8] = b"counter";

// Contract API
#[no_mangle]
pub extern "C" fn init() -> i32 {
    // Initialize storage on contract deployment
    storage::set(COUNTER_KEY, &0u32.to_le_bytes());
    0 // Success
}

#[no_mangle]
pub extern "C" fn get_counter() -> i32 {
    let counter_bytes = storage::get(COUNTER_KEY).unwrap_or_default();
    let counter = if counter_bytes.len() == 4 {
        u32::from_le_bytes([counter_bytes[0], counter_bytes[1], counter_bytes[2], counter_bytes[3]])
    } else {
        0
    };
    
    // Write the counter value to output buffer
    let output = counter.to_le_bytes();
    write_result(&output);
    0 // Success
}

#[no_mangle]
pub extern "C" fn increment() -> i32 {
    // Read current counter value
    let counter_bytes = storage::get(COUNTER_KEY).unwrap_or_default();
    let mut counter = if counter_bytes.len() == 4 {
        u32::from_le_bytes([counter_bytes[0], counter_bytes[1], counter_bytes[2], counter_bytes[3]])
    } else {
        0
    };
    
    // Increment counter
    counter = counter.saturating_add(1);
    
    // Store updated value
    storage::set(COUNTER_KEY, &counter.to_le_bytes());
    0 // Success
}
"#;

/// Compile Rust code to WebAssembly
pub fn compile_rust(
    source_code: &str,
    project_dir: &Path,
    config: &CompilerConfig,
) -> CompilerResult<Vec<u8>> {
    // Create Cargo.toml
    let cargo_toml = CARGO_TOML_TEMPLATE
        .replace("{{package_name}}", project_dir.file_name().unwrap().to_str().unwrap());
    let cargo_toml_path = project_dir.join("Cargo.toml");
    let mut cargo_file = File::create(&cargo_toml_path)
        .map_err(|e| CompilerError::Compilation(format!("Failed to create Cargo.toml: {}", e)))?;
    cargo_file.write_all(cargo_toml.as_bytes())
        .map_err(|e| CompilerError::Compilation(format!("Failed to write Cargo.toml: {}", e)))?;
    
    // Create source directory
    let src_dir = project_dir.join("src");
    if !src_dir.exists() {
        fs::create_dir_all(&src_dir)
            .map_err(|e| CompilerError::Compilation(format!("Failed to create src directory: {}", e)))?;
    }
    
    // Use template if source code is empty
    let source = if source_code.trim().is_empty() {
        SOURCE_TEMPLATE
    } else {
        source_code
    };
    
    // Write source code to lib.rs
    let lib_rs_path = src_dir.join("lib.rs");
    let mut lib_file = File::create(&lib_rs_path)
        .map_err(|e| CompilerError::Compilation(format!("Failed to create lib.rs: {}", e)))?;
    lib_file.write_all(source.as_bytes())
        .map_err(|e| CompilerError::Compilation(format!("Failed to write lib.rs: {}", e)))?;
    
    // Generate config.toml for cargo
    generate_cargo_config(project_dir, config)?;
    
    // Compile with timeout
    let start_time = Instant::now();
    let compilation_timeout = Duration::from_secs(config.max_compilation_time);
    
    // Create a thread for compilation
    let project_dir_clone = project_dir.to_path_buf();
    let compilation_thread = thread::spawn(move || {
        compile_cargo_project(&project_dir_clone)
    });
    
    // Wait for compilation with timeout
    let compilation_result = match compilation_thread.join() {
        Ok(result) => result,
        Err(_) => Err(CompilerError::Compilation("Compilation thread panicked".to_string())),
    };
    
    // Check timeout
    if start_time.elapsed() > compilation_timeout {
        return Err(CompilerError::Compilation(
            format!("Compilation timeout after {} seconds", config.max_compilation_time)
        ));
    }
    
    // Get the compilation result
    let wasm_path = compilation_result?;
    
    // Read the WebAssembly binary
    let wasm_binary = fs::read(&wasm_path)
        .map_err(|e| CompilerError::Compilation(format!("Failed to read WASM binary: {}", e)))?;
    
    // Check binary size
    if wasm_binary.len() > config.max_binary_size {
        return Err(CompilerError::Compilation(
            format!("WASM binary too large: {} bytes (max: {} bytes)", 
                wasm_binary.len(), config.max_binary_size)
        ));
    }
    
    Ok(wasm_binary)
}

/// Compile a Cargo project
fn compile_cargo_project(project_dir: &Path) -> CompilerResult<PathBuf> {
    // Run cargo build
    let output = Command::new("cargo")
        .args(&["+nightly", "build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| CompilerError::Compilation(format!("Failed to run cargo: {}", e)))?;
    
    // Check if compilation succeeded
    if !output.status.success() {
        return Err(CompilerError::Compilation(
            format!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    // Get the output WASM file
    let wasm_path = project_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(project_dir.file_name().unwrap())
        .with_extension("wasm");
    
    // Check if WASM file exists
    if !wasm_path.exists() {
        return Err(CompilerError::Compilation("WASM file not found after compilation".to_string()));
    }
    
    Ok(wasm_path)
}

/// Generate Cargo config.toml
fn generate_cargo_config(project_dir: &Path, config: &CompilerConfig) -> CompilerResult<()> {
    let config_dir = project_dir.join(".cargo");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| CompilerError::Compilation(format!("Failed to create .cargo directory: {}", e)))?;
    }
    
    // Define optimization flags based on config
    let opt_level = match config.optimization_level {
        crate::contracts::compiler::OptimizationLevel::None => "\"0\"",
        crate::contracts::compiler::OptimizationLevel::Basic => "\"1\"",
        crate::contracts::compiler::OptimizationLevel::Size => "\"z\"",
        crate::contracts::compiler::OptimizationLevel::Speed => "\"3\"",
    };
    
    // Create config.toml content
    let config_content = format!(r#"
[build]
target = "wasm32-unknown-unknown"

[target.wasm32-unknown-unknown]
rustflags = [
    "-C", "opt-level={}",
    "-C", "link-arg=--import-memory",
    "-C", "link-arg=--initial-memory={},{}",
]
"#, 
        opt_level, 
        config.max_memory_pages,
        if config.stack_protection { "\n    \"-C\", \"link-arg=--stack-first\"," } else { "" }
    );
    
    // Write config.toml
    let config_path = config_dir.join("config.toml");
    let mut config_file = File::create(&config_path)
        .map_err(|e| CompilerError::Compilation(format!("Failed to create config.toml: {}", e)))?;
    config_file.write_all(config_content.as_bytes())
        .map_err(|e| CompilerError::Compilation(format!("Failed to write config.toml: {}", e)))?;
    
    Ok(())
}

/// Check if the Rust toolchain is properly installed
pub fn check_rust_installation() -> CompilerResult<()> {
    // Check if rustc is installed
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| CompilerError::MissingDependency(format!("Failed to run rustc: {}", e)))?;
    
    if !output.status.success() {
        return Err(CompilerError::MissingDependency("Rust compiler (rustc) not installed".to_string()));
    }
    
    // Check if cargo is installed
    let output = Command::new("cargo")
        .arg("--version")
        .output()
        .map_err(|e| CompilerError::MissingDependency(format!("Failed to run cargo: {}", e)))?;
    
    if !output.status.success() {
        return Err(CompilerError::MissingDependency("Cargo not installed".to_string()));
    }
    
    // Check if wasm32 target is installed
    let output = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()
        .map_err(|e| CompilerError::MissingDependency(format!("Failed to run rustup: {}", e)))?;
    
    let installed_targets = String::from_utf8_lossy(&output.stdout);
    if !installed_targets.contains("wasm32-unknown-unknown") {
        return Err(CompilerError::MissingDependency(
            "wasm32-unknown-unknown target not installed. Run 'rustup target add wasm32-unknown-unknown'".to_string()
        ));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_generate_cargo_config() {
        let temp_dir = tempdir().unwrap();
        let config = CompilerConfig::default();
        
        // Generate cargo config
        let result = generate_cargo_config(temp_dir.path(), &config);
        assert!(result.is_ok());
        
        // Check if file was created
        let config_path = temp_dir.path().join(".cargo").join("config.toml");
        assert!(config_path.exists());
    }
} 