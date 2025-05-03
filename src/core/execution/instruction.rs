/// OpCode enum for the World Ledger VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    // Stack operations
    PUSH,    // Push value onto stack
    POP,     // Pop value from stack
    DUP,     // Duplicate top of stack
    SWAP,    // Swap top two stack items
    
    // Arithmetic operations
    ADD,     // Addition
    SUB,     // Subtraction
    MUL,     // Multiplication
    DIV,     // Division
    MOD,     // Modulus
    
    // Bitwise operations
    AND,     // Bitwise AND
    OR,      // Bitwise OR
    XOR,     // Bitwise XOR
    NOT,     // Bitwise NOT
    SHL,     // Shift left
    SHR,     // Shift right
    
    // Control flow
    JUMP,    // Unconditional jump
    JUMPI,   // Conditional jump
    CALL,    // Call function
    RETURN,  // Return from function
    STOP,    // Stop execution
    
    // Memory operations
    MLOAD,   // Load from memory
    MSTORE,  // Store to memory
    
    // Storage operations
    SLOAD,   // Load from storage
    SSTORE,  // Store to storage
    
    // External operations
    CREATE,  // Create new contract
    CALL_EXT,// Call external contract
    LOG,     // Log event
}

impl OpCode {
    /// Get the gas cost of this opcode
    pub fn gas_cost(&self) -> u64 {
        match self {
            // Stack operations - cheap
            OpCode::PUSH => 3,
            OpCode::POP => 2,
            OpCode::DUP => 3,
            OpCode::SWAP => 3,
            
            // Arithmetic operations - medium cost
            OpCode::ADD => 5,
            OpCode::SUB => 5,
            OpCode::MUL => 10,
            OpCode::DIV => 15,
            OpCode::MOD => 15,
            
            // Bitwise operations - medium cost
            OpCode::AND => 5,
            OpCode::OR => 5,
            OpCode::XOR => 5,
            OpCode::NOT => 5,
            OpCode::SHL => 10,
            OpCode::SHR => 10,
            
            // Control flow - varied cost
            OpCode::JUMP => 8,
            OpCode::JUMPI => 10,
            OpCode::CALL => 100,
            OpCode::RETURN => 0,  // Final operation, no gas
            OpCode::STOP => 0,    // Final operation, no gas
            
            // Memory operations - variable based on size
            OpCode::MLOAD => 3,   // Base cost, size adds more
            OpCode::MSTORE => 6,  // Base cost, size adds more
            
            // Storage operations - expensive
            OpCode::SLOAD => 200,
            OpCode::SSTORE => 5000,
            
            // External operations - very expensive
            OpCode::CREATE => 32000,
            OpCode::CALL_EXT => 700,
            OpCode::LOG => 375,
        }
    }
    
    /// Get the byte representation of this opcode
    pub fn to_byte(&self) -> u8 {
        match self {
            OpCode::PUSH => 0x60,
            OpCode::POP => 0x50,
            OpCode::DUP => 0x80,
            OpCode::SWAP => 0x90,
            
            OpCode::ADD => 0x01,
            OpCode::SUB => 0x03,
            OpCode::MUL => 0x02,
            OpCode::DIV => 0x04,
            OpCode::MOD => 0x06,
            
            OpCode::AND => 0x16,
            OpCode::OR => 0x17,
            OpCode::XOR => 0x18,
            OpCode::NOT => 0x19,
            OpCode::SHL => 0x1B,
            OpCode::SHR => 0x1C,
            
            OpCode::JUMP => 0x56,
            OpCode::JUMPI => 0x57,
            OpCode::CALL => 0xF1,
            OpCode::RETURN => 0xF3,
            OpCode::STOP => 0x00,
            
            OpCode::MLOAD => 0x51,
            OpCode::MSTORE => 0x52,
            
            OpCode::SLOAD => 0x54,
            OpCode::SSTORE => 0x55,
            
            OpCode::CREATE => 0xF0,
            OpCode::CALL_EXT => 0xF2,
            OpCode::LOG => 0xA0,
        }
    }
    
    /// Convert from byte to opcode
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x60 => Some(OpCode::PUSH),
            0x50 => Some(OpCode::POP),
            0x80 => Some(OpCode::DUP),
            0x90 => Some(OpCode::SWAP),
            
            0x01 => Some(OpCode::ADD),
            0x03 => Some(OpCode::SUB),
            0x02 => Some(OpCode::MUL),
            0x04 => Some(OpCode::DIV),
            0x06 => Some(OpCode::MOD),
            
            0x16 => Some(OpCode::AND),
            0x17 => Some(OpCode::OR),
            0x18 => Some(OpCode::XOR),
            0x19 => Some(OpCode::NOT),
            0x1B => Some(OpCode::SHL),
            0x1C => Some(OpCode::SHR),
            
            0x56 => Some(OpCode::JUMP),
            0x57 => Some(OpCode::JUMPI),
            0xF1 => Some(OpCode::CALL),
            0xF3 => Some(OpCode::RETURN),
            0x00 => Some(OpCode::STOP),
            
            0x51 => Some(OpCode::MLOAD),
            0x52 => Some(OpCode::MSTORE),
            
            0x54 => Some(OpCode::SLOAD),
            0x55 => Some(OpCode::SSTORE),
            
            0xF0 => Some(OpCode::CREATE),
            0xF2 => Some(OpCode::CALL_EXT),
            0xA0 => Some(OpCode::LOG),
            
            _ => None,
        }
    }
} 