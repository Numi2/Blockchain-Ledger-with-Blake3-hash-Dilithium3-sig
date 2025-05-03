mod rust;
mod c;
mod assembly;
mod optimizer;
mod verification;

use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use tracing::{debug, error, info, warn};

/// Contract compiler error
#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Compilation error: {0}")]
    Compilation(String),
    
    #[error("Verification error: {0}")]
    Verification(String),
    
    #[error("Optimization error: {0}")]
    Optimization(String),
    
    #[error("Invalid source: {0}")]
    InvalidSource(String),
    
    #[error("Unknown language: {0}")]
    UnknownLanguage(String),
    
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
}

/// Result type for compiler operations
pub type CompilerResult<T> = Result<T, CompilerError>;

/// Supported source language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceLanguage {
    /// Rust language
    Rust,
    
    /// C language
    C,
    
    /// WebAssembly text format
    Wat,
    
    /// Assembly
    Assembly,
}

impl std::fmt::Display for SourceLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::C => write!(f, "c"),
            Self::Wat => write!(f, "wat"),
            Self::Assembly => write!(f, "assembly"),
        }
    }
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization (debugging)
    None,
    
    /// Basic optimizations
    Basic,
    
    /// Size optimizations
    Size,
    
    /// Speed optimizations
    Speed,
}

impl std::fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Basic => write!(f, "basic"),
            Self::Size => write!(f, "size"),
            Self::Speed => write!(f, "speed"),
        }
    }
}

/// Compiler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// Maximum memory pages (64KB each)
    pub max_memory_pages: u32,
    
    /// Maximum binary size in bytes
    pub max_binary_size: usize,
    
    /// Maximum compilation time in seconds
    pub max_compilation_time: u64,
    
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    
    /// Enable debugging information
    pub debug_info: bool,
    
    /// Enable gas metering
    pub gas_metering: bool,
    
    /// Enable stack protection
    pub stack_protection: bool,
    
    /// Runtime API version to target
    pub api_version: u32,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 100, // 6.4 MB
            max_binary_size: 1024 * 1024, // 1 MB
            max_compilation_time: 60, // 1 minute
            optimization_level: OptimizationLevel::Size,
            debug_info: false,
            gas_metering: true,
            stack_protection: true,
            api_version: 1,
        }
    }
}

/// Compiler output
#[derive(Debug, Clone)]
pub struct CompilerOutput {
    /// WebAssembly binary
    pub wasm_binary: Vec<u8>,
    
    /// WebAssembly text representation
    pub wasm_text: Option<String>,
    
    /// ABI definition
    pub abi: Option<String>,
    
    /// Debug information
    pub debug_info: Option<Vec<u8>>,
    
    /// Compilation warnings
    pub warnings: Vec<String>,
    
    /// Gas estimates for functions
    pub gas_estimates: Option<Vec<(String, u64)>>,
}

/// Contract compiler
pub struct ContractCompiler {
    /// Compiler configuration
    config: CompilerConfig,
    
    /// Working directory
    work_dir: PathBuf,
}

impl ContractCompiler {
    /// Create a new contract compiler
    pub fn new(config: CompilerConfig, work_dir: impl AsRef<Path>) -> CompilerResult<Self> {
        let work_dir = work_dir.as_ref().to_path_buf();
        
        // Create working directory if it doesn't exist
        if !work_dir.exists() {
            fs::create_dir_all(&work_dir)?;
        }
        
        Ok(Self {
            config,
            work_dir,
        })
    }
    
    /// Compile source code to WebAssembly
    pub fn compile(
        &self,
        source_code: &str,
        language: SourceLanguage,
        name: &str,
    ) -> CompilerResult<CompilerOutput> {
        // Create a subdirectory for this compilation
        let project_dir = self.work_dir.join(name);
        if !project_dir.exists() {
            fs::create_dir_all(&project_dir)?;
        }
        
        // Compile based on language
        let binary = match language {
            SourceLanguage::Rust => rust::compile_rust(source_code, &project_dir, &self.config)?,
            SourceLanguage::C => c::compile_c(source_code, &project_dir, &self.config)?,
            SourceLanguage::Wat => assembly::compile_wat(source_code, &project_dir, &self.config)?,
            SourceLanguage::Assembly => assembly::compile_assembly(source_code, &project_dir, &self.config)?,
        };
        
        // Optimize the WebAssembly binary
        let (optimized_binary, warnings) = optimizer::optimize_wasm(
            &binary,
            self.config.optimization_level,
            self.config.gas_metering,
            self.config.max_memory_pages,
        )?;
        
        // Verify the WebAssembly binary
        verification::verify_wasm(&optimized_binary, &self.config)?;
        
        // Generate WebAssembly text representation
        let wasm_text = if self.config.debug_info {
            Some(self.generate_wasm_text(&optimized_binary)?)
        } else {
            None
        };
        
        // Extract ABI information (if available)
        let abi = self.extract_abi(&optimized_binary)?;
        
        // Extract debug information (if available)
        let debug_info = if self.config.debug_info {
            Some(self.extract_debug_info(&optimized_binary)?)
        } else {
            None
        };
        
        // Estimate gas usage for functions
        let gas_estimates = if self.config.gas_metering {
            Some(self.estimate_gas_usage(&optimized_binary)?)
        } else {
            None
        };
        
        Ok(CompilerOutput {
            wasm_binary: optimized_binary,
            wasm_text,
            abi,
            debug_info,
            warnings,
            gas_estimates,
        })
    }
    
    /// Generate WebAssembly text representation
    fn generate_wasm_text(&self, wasm_binary: &[u8]) -> CompilerResult<String> {
        // Use wabt to convert binary to text
        let temp_file = self.work_dir.join("temp.wasm");
        fs::write(&temp_file, wasm_binary)?;
        
        let output = Command::new("wasm2wat")
            .arg(&temp_file)
            .output()
            .map_err(|e| CompilerError::Compilation(format!("Failed to run wasm2wat: {}", e)))?;
            
        if !output.status.success() {
            return Err(CompilerError::Compilation(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Clean up temporary file
        let _ = fs::remove_file(temp_file);
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Extract ABI information
    fn extract_abi(&self, wasm_binary: &[u8]) -> CompilerResult<Option<String>> {
        // This would normally parse the custom section for ABI
        // For now, return a placeholder
        Ok(None)
    }
    
    /// Extract debug information
    fn extract_debug_info(&self, wasm_binary: &[u8]) -> CompilerResult<Vec<u8>> {
        // This would normally extract the DWARF debug info
        // For now, return an empty vector
        Ok(Vec::new())
    }
    
    /// Estimate gas usage for functions
    fn estimate_gas_usage(&self, wasm_binary: &[u8]) -> CompilerResult<Vec<(String, u64)>> {
        // This would analyze the binary to estimate gas usage
        // For now, return an empty vector
        Ok(Vec::new())
    }
    
    /// Check if compilation dependencies are installed
    pub fn check_dependencies(&self) -> CompilerResult<()> {
        // Check for Rust compiler if needed
        if !Self::is_program_installed("rustc") {
            return Err(CompilerError::MissingDependency(
                "Rust compiler (rustc) not found in PATH".to_string()
            ));
        }
        
        // Check for wasm-opt if optimizations are enabled
        if self.config.optimization_level != OptimizationLevel::None 
            && !Self::is_program_installed("wasm-opt") {
            return Err(CompilerError::MissingDependency(
                "wasm-opt not found in PATH (from binaryen)".to_string()
            ));
        }
        
        // Check for wasm2wat if debug info is enabled
        if self.config.debug_info && !Self::is_program_installed("wasm2wat") {
            return Err(CompilerError::MissingDependency(
                "wasm2wat not found in PATH (from wabt)".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Check if a program is installed
    fn is_program_installed(program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_compiler_initialization() {
        let temp_dir = tempdir().unwrap();
        let config = CompilerConfig::default();
        
        let compiler = ContractCompiler::new(config, temp_dir.path());
        assert!(compiler.is_ok());
    }
    
    // More tests would be added for actual compilation
} 