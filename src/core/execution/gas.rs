/// GasMeter tracks gas usage and limits during execution
pub struct GasMeter {
    /// Gas limit for the current execution
    limit: u64,
    
    /// Gas used so far
    used: u64,
}

/// Errors that can occur during gas metering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GasError {
    /// Gas limit exceeded
    OutOfGas,
    
    /// Gas calculation overflow
    Overflow,
}

impl GasMeter {
    /// Create a new gas meter with the given limit
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            used: 0,
        }
    }
    
    /// Get the current gas limit
    pub fn limit(&self) -> u64 {
        self.limit
    }
    
    /// Get the gas used so far
    pub fn used(&self) -> u64 {
        self.used
    }
    
    /// Get the remaining gas
    pub fn remaining(&self) -> u64 {
        self.limit - self.used
    }
    
    /// Charge gas for an operation, returning an error if gas limit is exceeded
    pub fn charge(&mut self, amount: u64) -> Result<(), GasError> {
        // Check for overflow
        let new_used = self.used.checked_add(amount).ok_or(GasError::Overflow)?;
        
        // Check if we have enough gas
        if new_used > self.limit {
            return Err(GasError::OutOfGas);
        }
        
        // Update used gas
        self.used = new_used;
        Ok(())
    }
    
    /// Charge gas for memory expansion
    pub fn charge_memory(&mut self, current_size: u64, new_size: u64) -> Result<(), GasError> {
        // Don't charge if not expanding
        if new_size <= current_size {
            return Ok(());
        }
        
        // Simple model: 1 gas per byte of expansion
        let expansion = new_size - current_size;
        self.charge(expansion)
    }
    
    /// Refund unused gas at the end of execution
    pub fn refund(&mut self, amount: u64) -> Result<(), GasError> {
        if amount > self.used {
            self.used = 0;
        } else {
            self.used -= amount;
        }
        
        Ok(())
    }
} 