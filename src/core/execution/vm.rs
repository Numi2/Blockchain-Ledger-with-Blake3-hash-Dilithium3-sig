use crate::core::execution::{GasMeter, GasError, OpCode};
use crate::types::{Address, Hash};
use std::collections::HashMap;

/// VM execution errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    /// Out of gas
    OutOfGas,
    
    /// Stack underflow (not enough items on stack)
    StackUnderflow,
    
    /// Stack overflow (too many items on stack)
    StackOverflow,
    
    /// Invalid jump destination
    InvalidJump,
    
    /// Invalid opcode
    InvalidOpcode,
    
    /// Execution reverted
    Reverted,
    
    /// Arithmetic error (e.g., division by zero)
    ArithmeticError,
    
    /// Memory access error
    MemoryError,
    
    /// Storage access error
    StorageError,
}

/// Execution context for the VM
pub struct ExecutionContext {
    /// Address of the contract being executed
    pub address: Address,
    
    /// Address of the caller
    pub caller: Address,
    
    /// Value being transferred
    pub value: u64,
    
    /// Input data for the call
    pub input: Vec<u8>,
    
    /// Gas price for this execution
    pub gas_price: u64,
}

/// Result of VM execution
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    
    /// Gas used during execution
    pub gas_used: u64,
    
    /// Output data from execution
    pub output: Vec<u8>,
    
    /// Error if execution failed
    pub error: Option<VMError>,
}

/// Simple stack-based VM for World Ledger
pub struct VM {
    /// Program code
    code: Vec<u8>,
    
    /// Program counter
    pc: usize,
    
    /// Stack for execution
    stack: Vec<u64>,
    
    /// Memory space
    memory: Vec<u8>,
    
    /// Storage (persistent state)
    storage: HashMap<Hash, Hash>,
    
    /// Gas meter for this execution
    gas_meter: GasMeter,
    
    /// Execution context
    context: ExecutionContext,
    
    /// Return data buffer
    return_data: Vec<u8>,
}

impl VM {
    /// Create a new VM instance
    pub fn new(
        code: Vec<u8>, 
        gas_limit: u64, 
        context: ExecutionContext
    ) -> Self {
        Self {
            code,
            pc: 0,
            stack: Vec::with_capacity(1024),
            memory: Vec::with_capacity(1024),
            storage: HashMap::new(),
            gas_meter: GasMeter::new(gas_limit),
            context,
            return_data: Vec::new(),
        }
    }
    
    /// Execute the code in the VM
    pub fn execute(&mut self) -> ExecutionResult {
        // Loop until execution completes or errors
        loop {
            // Check if we've reached the end of the code
            if self.pc >= self.code.len() {
                break;
            }
            
            // Read the next opcode
            let op_byte = self.code[self.pc];
            let opcode = match OpCode::from_byte(op_byte) {
                Some(op) => op,
                None => {
                    return ExecutionResult {
                        success: false,
                        gas_used: self.gas_meter.used(),
                        output: Vec::new(),
                        error: Some(VMError::InvalidOpcode),
                    };
                }
            };
            
            // Charge gas for this opcode
            if let Err(_) = self.gas_meter.charge(opcode.gas_cost()) {
                return ExecutionResult {
                    success: false,
                    gas_used: self.gas_meter.used(),
                    output: Vec::new(),
                    error: Some(VMError::OutOfGas),
                };
            }
            
            // Increment PC
            self.pc += 1;
            
            // Execute the opcode
            match self.execute_opcode(opcode) {
                Ok(should_continue) => {
                    if !should_continue {
                        // Successful termination (STOP or RETURN)
                        break;
                    }
                }
                Err(err) => {
                    return ExecutionResult {
                        success: false,
                        gas_used: self.gas_meter.used(),
                        output: Vec::new(),
                        error: Some(err),
                    };
                }
            }
        }
        
        // Execution completed successfully
        ExecutionResult {
            success: true,
            gas_used: self.gas_meter.used(),
            output: self.return_data.clone(),
            error: None,
        }
    }
    
    /// Execute a single opcode
    fn execute_opcode(&mut self, opcode: OpCode) -> Result<bool, VMError> {
        match opcode {
            OpCode::PUSH => {
                // Read the next byte as the value to push
                if self.pc >= self.code.len() {
                    return Err(VMError::InvalidOpcode);
                }
                let value = self.code[self.pc] as u64;
                self.pc += 1;
                
                // Push to stack
                if self.stack.len() >= 1024 {
                    return Err(VMError::StackOverflow);
                }
                self.stack.push(value);
            }
            
            OpCode::POP => {
                // Pop from stack
                if self.stack.is_empty() {
                    return Err(VMError::StackUnderflow);
                }
                self.stack.pop();
            }
            
            OpCode::ADD => {
                // Pop two values, add, and push result
                if self.stack.len() < 2 {
                    return Err(VMError::StackUnderflow);
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(a.wrapping_add(b));
            }
            
            OpCode::SUB => {
                // Pop two values, subtract, and push result
                if self.stack.len() < 2 {
                    return Err(VMError::StackUnderflow);
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(a.wrapping_sub(b));
            }
            
            OpCode::MUL => {
                // Pop two values, multiply, and push result
                if self.stack.len() < 2 {
                    return Err(VMError::StackUnderflow);
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(a.wrapping_mul(b));
            }
            
            OpCode::DIV => {
                // Pop two values, divide, and push result
                if self.stack.len() < 2 {
                    return Err(VMError::StackUnderflow);
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                if b == 0 {
                    self.stack.push(0); // Division by zero returns 0
                } else {
                    self.stack.push(a / b);
                }
            }
            
            OpCode::STOP => {
                // Stop execution
                return Ok(false);
            }
            
            OpCode::RETURN => {
                // Return data from the stack
                if self.stack.len() < 2 {
                    return Err(VMError::StackUnderflow);
                }
                
                let offset = self.stack.pop().unwrap() as usize;
                let length = self.stack.pop().unwrap() as usize;
                
                // Ensure we have enough memory
                if offset + length > self.memory.len() {
                    return Err(VMError::MemoryError);
                }
                
                // Set return data
                self.return_data = self.memory[offset..offset + length].to_vec();
                
                // Stop execution
                return Ok(false);
            }
            
            // Implement other opcodes...
            // This is a minimal implementation for demonstration
            
            _ => {
                // For now, we'll just handle a few opcodes and error on others
                return Err(VMError::InvalidOpcode);
            }
        }
        
        // Continue execution
        Ok(true)
    }
} 