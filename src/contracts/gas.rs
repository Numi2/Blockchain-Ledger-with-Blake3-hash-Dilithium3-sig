use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

/// Gas metering errors
#[derive(Error, Debug)]
pub enum GasError {
    #[error("Out of gas")]
    OutOfGas,
}

/// Result type for gas operations
pub type GasResult<T> = Result<T, GasError>;

/// Gas configuration for different operations
#[derive(Debug, Clone)]
pub struct GasConfig {
    /// Base cost for storage read operations
    pub storage_read_base: u64,
    
    /// Per-byte cost for storage read
    pub storage_read_per_byte: u64,
    
    /// Base cost for storage write operations
    pub storage_write_base: u64,
    
    /// Per-byte cost for storage write
    pub storage_write_per_byte: u64,
    
    /// Base cost for external contract calls
    pub external_call_base: u64,
    
    /// Per-byte cost for external contract call data
    pub external_call_per_byte: u64,
    
    /// Base cost for log/event operations
    pub log_base: u64,
    
    /// Per-byte cost for log/event data
    pub log_per_byte: u64,
    
    /// Cost per WASM instruction (computational complexity)
    pub wasm_instruction: u64,
    
    /// Cost per memory page allocation
    pub memory_page: u64,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            storage_read_base: 100,
            storage_read_per_byte: 1,
            storage_write_base: 200,
            storage_write_per_byte: 2,
            external_call_base: 700,
            external_call_per_byte: 3,
            log_base: 100,
            log_per_byte: 1,
            wasm_instruction: 1,
            memory_page: 1000,
        }
    }
}

/// Gas meter for tracking execution costs
pub struct GasMeter {
    /// Gas limit
    limit: u64,
    
    /// Gas used
    used: AtomicU64,
}

impl GasMeter {
    /// Create a new gas meter with the given limit
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            used: AtomicU64::new(0),
        }
    }
    
    /// Get the current gas used
    pub fn gas_used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }
    
    /// Get the gas limit
    pub fn gas_limit(&self) -> u64 {
        self.limit
    }
    
    /// Get the remaining gas
    pub fn gas_remaining(&self) -> u64 {
        let used = self.gas_used();
        if used > self.limit {
            0
        } else {
            self.limit - used
        }
    }
    
    /// Charge gas for an operation
    pub fn charge(&self, amount: u64) -> GasResult<()> {
        // Get current gas usage and add the amount
        let prev_used = self.used.fetch_add(amount, Ordering::Relaxed);
        let new_used = prev_used.saturating_add(amount);
        
        // Check if we've exceeded the limit
        if new_used > self.limit {
            return Err(GasError::OutOfGas);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gas_metering() {
        let meter = GasMeter::new(1000);
        
        // Charge some gas
        assert!(meter.charge(300).is_ok());
        assert_eq!(meter.gas_used(), 300);
        assert_eq!(meter.gas_remaining(), 700);
        
        // Charge more gas
        assert!(meter.charge(400).is_ok());
        assert_eq!(meter.gas_used(), 700);
        assert_eq!(meter.gas_remaining(), 300);
        
        // Should fail when exceeding limit
        assert!(meter.charge(500).is_err());
        
        // Can still charge small amounts
        assert!(meter.charge(100).is_ok());
        assert_eq!(meter.gas_used(), 800);
        assert_eq!(meter.gas_remaining(), 200);
    }
} 