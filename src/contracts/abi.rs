use crate::types::Hash;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use thiserror::Error;

/// ABI error types
#[derive(Error, Debug)]
pub enum AbiError {
    #[error("Type mismatch: expected {0}, got {1}")]
    TypeMismatch(String, String),
    
    #[error("Invalid ABI: {0}")]
    InvalidAbi(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
    
    #[error("Decoding error: {0}")]
    DecodingError(String),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
}

/// ABI operation result
pub type AbiResult<T> = Result<T, AbiError>;

/// Parameter type in ABI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamType {
    /// Unsigned integer (8, 16, 32, 64 bits)
    Uint(usize),
    
    /// Signed integer (8, 16, 32, 64 bits)
    Int(usize),
    
    /// Boolean
    Bool,
    
    /// String
    String,
    
    /// Bytes (fixed size or dynamic)
    Bytes(Option<usize>),
    
    /// Address (20 bytes)
    Address,
    
    /// Array of items (fixed size or dynamic)
    Array(Box<ParamType>, Option<usize>),
    
    /// Hash (32 bytes)
    Hash,
}

impl std::fmt::Display for ParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamType::Uint(size) => write!(f, "uint{}", size),
            ParamType::Int(size) => write!(f, "int{}", size),
            ParamType::Bool => write!(f, "bool"),
            ParamType::String => write!(f, "string"),
            ParamType::Bytes(None) => write!(f, "bytes"),
            ParamType::Bytes(Some(size)) => write!(f, "bytes{}", size),
            ParamType::Address => write!(f, "address"),
            ParamType::Array(inner, None) => write!(f, "{}[]", inner),
            ParamType::Array(inner, Some(size)) => write!(f, "{}[{}]", inner, size),
            ParamType::Hash => write!(f, "hash"),
        }
    }
}

/// Function parameter in ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    /// Parameter name
    pub name: String,
    
    /// Parameter type
    pub param_type: ParamType,
}

/// Function in ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Function name
    pub name: String,
    
    /// Function inputs
    pub inputs: Vec<Param>,
    
    /// Function outputs
    pub outputs: Vec<Param>,
    
    /// Is this a view function (read-only)
    pub is_view: bool,
}

/// Event parameter in ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParam {
    /// Parameter name
    pub name: String,
    
    /// Parameter type
    pub param_type: ParamType,
    
    /// Is this parameter indexed (searchable)
    pub indexed: bool,
}

/// Event in ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event name
    pub name: String,
    
    /// Event parameters
    pub params: Vec<EventParam>,
}

/// Contract ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAbi {
    /// Contract functions
    pub functions: Vec<Function>,
    
    /// Contract events
    pub events: Vec<Event>,
}

impl ContractAbi {
    /// Parse ABI from JSON
    pub fn from_json(json: &str) -> AbiResult<Self> {
        let value: Value = serde_json::from_str(json)
            .map_err(|e| AbiError::InvalidAbi(format!("Failed to parse JSON: {}", e)))?;
        
        let mut functions = Vec::new();
        let mut events = Vec::new();
        
        let array = value.as_array()
            .ok_or_else(|| AbiError::InvalidAbi("ABI must be an array".to_string()))?;
        
        for item in array {
            let item_obj = item.as_object()
                .ok_or_else(|| AbiError::InvalidAbi("ABI item must be an object".to_string()))?;
            
            let item_type = item_obj.get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| AbiError::InvalidAbi("ABI item missing 'type' field".to_string()))?;
            
            match item_type {
                "function" => {
                    let name = item_obj.get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| AbiError::InvalidAbi("Function missing 'name' field".to_string()))?
                        .to_string();
                    
                    let inputs = parse_params(item_obj.get("inputs"))?;
                    let outputs = parse_params(item_obj.get("outputs"))?;
                    
                    let stateMutability = item_obj.get("stateMutability")
                        .and_then(|s| s.as_str())
                        .unwrap_or("nonpayable");
                    
                    let is_view = stateMutability == "view" || stateMutability == "pure";
                    
                    functions.push(Function {
                        name,
                        inputs,
                        outputs,
                        is_view,
                    });
                },
                "event" => {
                    let name = item_obj.get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| AbiError::InvalidAbi("Event missing 'name' field".to_string()))?
                        .to_string();
                    
                    let params = parse_event_params(item_obj.get("inputs"))?;
                    
                    events.push(Event {
                        name,
                        params,
                    });
                },
                _ => {} // Skip other types
            }
        }
        
        Ok(ContractAbi {
            functions,
            events,
        })
    }
    
    /// Find function by name
    pub fn find_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }
    
    /// Find event by name
    pub fn find_event(&self, name: &str) -> Option<&Event> {
        self.events.iter().find(|e| e.name == name)
    }
    
    /// Compute function selector (first 4 bytes of function signature hash)
    pub fn function_selector(&self, name: &str) -> AbiResult<[u8; 4]> {
        let function = self.find_function(name)
            .ok_or_else(|| AbiError::FunctionNotFound(name.to_string()))?;
        
        // Build function signature (name + types)
        let sig = build_function_signature(function);
        
        // Hash the signature
        let hash = compute_signature_hash(&sig);
        
        // Return first 4 bytes
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&hash[0..4]);
        
        Ok(selector)
    }
    
    /// Encode function call with parameters
    pub fn encode_function_call(&self, name: &str, args: &[Value]) -> AbiResult<Vec<u8>> {
        let function = self.find_function(name)
            .ok_or_else(|| AbiError::FunctionNotFound(name.to_string()))?;
        
        // Check argument count
        if args.len() != function.inputs.len() {
            return Err(AbiError::InvalidParameter(format!(
                "Function '{}' expects {} arguments, got {}",
                name, function.inputs.len(), args.len()
            )));
        }
        
        // Get function selector
        let selector = self.function_selector(name)?;
        
        // Encode arguments
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&selector);
        
        for (i, arg) in args.iter().enumerate() {
            let param_type = &function.inputs[i].param_type;
            let encoded_arg = encode_param(param_type, arg)?;
            encoded.extend(encoded_arg);
        }
        
        Ok(encoded)
    }
    
    /// Decode function output
    pub fn decode_function_output(&self, name: &str, data: &[u8]) -> AbiResult<Vec<Value>> {
        let function = self.find_function(name)
            .ok_or_else(|| AbiError::FunctionNotFound(name.to_string()))?;
        
        let mut offset = 0;
        let mut result = Vec::new();
        
        for output in &function.outputs {
            let (value, bytes_read) = decode_param(&output.param_type, &data[offset..])?;
            result.push(value);
            offset += bytes_read;
        }
        
        Ok(result)
    }
}

/// Parse parameters from ABI JSON
fn parse_params(params_value: Option<&Value>) -> AbiResult<Vec<Param>> {
    let mut params = Vec::new();
    
    if let Some(params_array) = params_value.and_then(|p| p.as_array()) {
        for param_value in params_array {
            let param_obj = param_value.as_object()
                .ok_or_else(|| AbiError::InvalidAbi("Parameter must be an object".to_string()))?;
            
            let name = param_obj.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            
            let type_str = param_obj.get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| AbiError::InvalidAbi("Parameter missing 'type' field".to_string()))?;
            
            let param_type = parse_type(type_str)?;
            
            params.push(Param {
                name,
                param_type,
            });
        }
    }
    
    Ok(params)
}

/// Parse event parameters from ABI JSON
fn parse_event_params(params_value: Option<&Value>) -> AbiResult<Vec<EventParam>> {
    let mut params = Vec::new();
    
    if let Some(params_array) = params_value.and_then(|p| p.as_array()) {
        for param_value in params_array {
            let param_obj = param_value.as_object()
                .ok_or_else(|| AbiError::InvalidAbi("Parameter must be an object".to_string()))?;
            
            let name = param_obj.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            
            let type_str = param_obj.get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| AbiError::InvalidAbi("Parameter missing 'type' field".to_string()))?;
            
            let indexed = param_obj.get("indexed")
                .and_then(|i| i.as_bool())
                .unwrap_or(false);
            
            let param_type = parse_type(type_str)?;
            
            params.push(EventParam {
                name,
                param_type,
                indexed,
            });
        }
    }
    
    Ok(params)
}

/// Parse a type string into a ParamType
fn parse_type(type_str: &str) -> AbiResult<ParamType> {
    if type_str.starts_with("uint") {
        let size_str = &type_str[4..];
        let size = size_str.parse::<usize>()
            .map_err(|_| AbiError::InvalidAbi(format!("Invalid uint size: {}", size_str)))?;
        
        return Ok(ParamType::Uint(size));
    }
    
    if type_str.starts_with("int") {
        let size_str = &type_str[3..];
        let size = size_str.parse::<usize>()
            .map_err(|_| AbiError::InvalidAbi(format!("Invalid int size: {}", size_str)))?;
        
        return Ok(ParamType::Int(size));
    }
    
    if type_str == "bool" {
        return Ok(ParamType::Bool);
    }
    
    if type_str == "string" {
        return Ok(ParamType::String);
    }
    
    if type_str == "address" {
        return Ok(ParamType::Address);
    }
    
    if type_str == "bytes" {
        return Ok(ParamType::Bytes(None));
    }
    
    if type_str.starts_with("bytes") {
        let size_str = &type_str[5..];
        let size = size_str.parse::<usize>()
            .map_err(|_| AbiError::InvalidAbi(format!("Invalid bytes size: {}", size_str)))?;
        
        return Ok(ParamType::Bytes(Some(size)));
    }
    
    if type_str == "hash" || type_str == "bytes32" {
        return Ok(ParamType::Hash);
    }
    
    if type_str.ends_with("[]") {
        let inner_type = &type_str[0..type_str.len() - 2];
        let inner = parse_type(inner_type)?;
        return Ok(ParamType::Array(Box::new(inner), None));
    }
    
    if let Some(pos) = type_str.find('[') {
        if type_str.ends_with(']') {
            let inner_type = &type_str[0..pos];
            let size_str = &type_str[pos + 1..type_str.len() - 1];
            let size = size_str.parse::<usize>()
                .map_err(|_| AbiError::InvalidAbi(format!("Invalid array size: {}", size_str)))?;
            
            let inner = parse_type(inner_type)?;
            return Ok(ParamType::Array(Box::new(inner), Some(size)));
        }
    }
    
    Err(AbiError::InvalidAbi(format!("Unsupported type: {}", type_str)))
}

/// Build a function signature (name + param types)
fn build_function_signature(function: &Function) -> String {
    let mut sig = function.name.clone();
    sig.push('(');
    
    for (i, param) in function.inputs.iter().enumerate() {
        if i > 0 {
            sig.push(',');
        }
        sig.push_str(&param.param_type.to_string());
    }
    
    sig.push(')');
    sig
}

/// Compute the hash of a function signature
fn compute_signature_hash(signature: &str) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    
    let mut hasher = Keccak256::new();
    hasher.update(signature.as_bytes());
    let result = hasher.finalize();
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Encode a parameter value
fn encode_param(param_type: &ParamType, value: &Value) -> AbiResult<Vec<u8>> {
    match param_type {
        ParamType::Uint(size) => encode_uint(*size, value),
        ParamType::Int(size) => encode_int(*size, value),
        ParamType::Bool => encode_bool(value),
        ParamType::String => encode_string(value),
        ParamType::Bytes(size) => encode_bytes(*size, value),
        ParamType::Address => encode_address(value),
        ParamType::Array(inner, size) => encode_array(inner, *size, value),
        ParamType::Hash => encode_hash(value),
    }
}

/// Encode an unsigned integer
fn encode_uint(size: usize, value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is a number
    let num = value.as_u64().ok_or_else(|| 
        AbiError::TypeMismatch(format!("uint{}", size), value.to_string()))?;
    
    // Check size and create result
    let num_bytes = size / 8;
    let mut result = Vec::with_capacity(num_bytes);
    
    // Fill with bytes
    for i in 0..num_bytes {
        let byte = (num >> (8 * (num_bytes - 1 - i))) as u8;
        result.push(byte);
    }
    
    Ok(result)
}

/// Encode a signed integer
fn encode_int(size: usize, value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is a number
    let num = if let Some(n) = value.as_i64() {
        n
    } else if let Some(n) = value.as_u64() {
        n as i64 // safe if n is actually a positive number
    } else {
        return Err(AbiError::TypeMismatch(format!("int{}", size), value.to_string()));
    };
    
    // Check size and create result
    let num_bytes = size / 8;
    let mut result = Vec::with_capacity(num_bytes);
    
    // Fill with bytes
    for i in 0..num_bytes {
        let byte = (num >> (8 * (num_bytes - 1 - i))) as u8;
        result.push(byte);
    }
    
    Ok(result)
}

/// Encode a boolean
fn encode_bool(value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is a boolean
    let b = value.as_bool().ok_or_else(|| 
        AbiError::TypeMismatch("bool".to_string(), value.to_string()))?;
    
    Ok(vec![if b { 1 } else { 0 }])
}

/// Encode a string
fn encode_string(value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is a string
    let s = value.as_str().ok_or_else(|| 
        AbiError::TypeMismatch("string".to_string(), value.to_string()))?;
    
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() + 4);
    
    // Add length prefix (4 bytes)
    result.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    
    // Add string bytes
    result.extend_from_slice(bytes);
    
    Ok(result)
}

/// Encode bytes
fn encode_bytes(size: Option<usize>, value: &Value) -> AbiResult<Vec<u8>> {
    let bytes = if let Some(s) = value.as_str() {
        // Handle hex strings
        if s.starts_with("0x") {
            hex::decode(&s[2..]).map_err(|e| 
                AbiError::EncodingError(format!("Invalid hex string: {}", e)))?
        } else {
            s.as_bytes().to_vec()
        }
    } else if let Some(array) = value.as_array() {
        // Handle array of numbers
        let mut bytes = Vec::with_capacity(array.len());
        for item in array {
            let byte = item.as_u64().ok_or_else(|| 
                AbiError::TypeMismatch("byte".to_string(), item.to_string()))?;
            
            if byte > 255 {
                return Err(AbiError::EncodingError(format!("Byte value out of range: {}", byte)));
            }
            
            bytes.push(byte as u8);
        }
        bytes
    } else {
        return Err(AbiError::TypeMismatch("bytes".to_string(), value.to_string()));
    };
    
    // Fixed size bytes
    if let Some(size) = size {
        if bytes.len() != size {
            return Err(AbiError::EncodingError(format!(
                "Expected {} bytes, got {}", size, bytes.len()
            )));
        }
        
        Ok(bytes)
    } else {
        // Dynamic size bytes with length prefix
        let mut result = Vec::with_capacity(bytes.len() + 4);
        
        // Add length prefix (4 bytes)
        result.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        
        // Add bytes
        result.extend_from_slice(&bytes);
        
        Ok(result)
    }
}

/// Encode an address
fn encode_address(value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is a string
    let s = value.as_str().ok_or_else(|| 
        AbiError::TypeMismatch("address".to_string(), value.to_string()))?;
    
    // Remove 0x prefix
    let s = if s.starts_with("0x") { &s[2..] } else { s };
    
    // Decode hex
    let bytes = hex::decode(s).map_err(|e| 
        AbiError::EncodingError(format!("Invalid address: {}", e)))?;
    
    // Check length
    if bytes.len() != 20 {
        return Err(AbiError::EncodingError(format!(
            "Address must be 20 bytes, got {}", bytes.len()
        )));
    }
    
    Ok(bytes)
}

/// Encode an array
fn encode_array(inner: &ParamType, size: Option<usize>, value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is an array
    let array = value.as_array().ok_or_else(|| 
        AbiError::TypeMismatch("array".to_string(), value.to_string()))?;
    
    // Check fixed array size
    if let Some(expected_size) = size {
        if array.len() != expected_size {
            return Err(AbiError::EncodingError(format!(
                "Expected array of size {}, got {}", expected_size, array.len()
            )));
        }
    }
    
    let mut result = Vec::new();
    
    // For dynamic arrays, add length prefix
    if size.is_none() {
        result.extend_from_slice(&(array.len() as u32).to_be_bytes());
    }
    
    // Encode each element
    for item in array {
        let encoded = encode_param(inner, item)?;
        result.extend(encoded);
    }
    
    Ok(result)
}

/// Encode a hash
fn encode_hash(value: &Value) -> AbiResult<Vec<u8>> {
    // Check value is a string
    let s = value.as_str().ok_or_else(|| 
        AbiError::TypeMismatch("hash".to_string(), value.to_string()))?;
    
    // Remove 0x prefix
    let s = if s.starts_with("0x") { &s[2..] } else { s };
    
    // Decode hex
    let bytes = hex::decode(s).map_err(|e| 
        AbiError::EncodingError(format!("Invalid hash: {}", e)))?;
    
    // Check length
    if bytes.len() != 32 {
        return Err(AbiError::EncodingError(format!(
            "Hash must be 32 bytes, got {}", bytes.len()
        )));
    }
    
    Ok(bytes)
}

/// Decode a parameter value
fn decode_param(param_type: &ParamType, data: &[u8]) -> AbiResult<(Value, usize)> {
    match param_type {
        ParamType::Uint(size) => decode_uint(*size, data),
        ParamType::Int(size) => decode_int(*size, data),
        ParamType::Bool => decode_bool(data),
        ParamType::String => decode_string(data),
        ParamType::Bytes(size) => decode_bytes(*size, data),
        ParamType::Address => decode_address(data),
        ParamType::Array(inner, size) => decode_array(inner, *size, data),
        ParamType::Hash => decode_hash(data),
    }
}

/// Decode an unsigned integer
fn decode_uint(size: usize, data: &[u8]) -> AbiResult<(Value, usize)> {
    let num_bytes = size / 8;
    
    if data.len() < num_bytes {
        return Err(AbiError::DecodingError(format!(
            "Not enough data for uint{}: need {} bytes, got {}", 
            size, num_bytes, data.len()
        )));
    }
    
    let mut value: u64 = 0;
    for i in 0..num_bytes {
        value = (value << 8) | data[i] as u64;
    }
    
    Ok((Value::Number(value.into()), num_bytes))
}

/// Decode a signed integer
fn decode_int(size: usize, data: &[u8]) -> AbiResult<(Value, usize)> {
    let num_bytes = size / 8;
    
    if data.len() < num_bytes {
        return Err(AbiError::DecodingError(format!(
            "Not enough data for int{}: need {} bytes, got {}", 
            size, num_bytes, data.len()
        )));
    }
    
    // Check if negative (highest bit set)
    let is_negative = (data[0] & 0x80) != 0;
    
    let mut value: i64 = 0;
    for i in 0..num_bytes {
        value = (value << 8) | data[i] as i64;
    }
    
    // Handle negative values
    if is_negative {
        value = value - (1i64 << (num_bytes * 8));
    }
    
    Ok((Value::Number(value.into()), num_bytes))
}

/// Decode a boolean
fn decode_bool(data: &[u8]) -> AbiResult<(Value, usize)> {
    if data.is_empty() {
        return Err(AbiError::DecodingError("Not enough data for bool".to_string()));
    }
    
    let value = data[0] != 0;
    Ok((Value::Bool(value), 1))
}

/// Decode a string
fn decode_string(data: &[u8]) -> AbiResult<(Value, usize)> {
    if data.len() < 4 {
        return Err(AbiError::DecodingError("Not enough data for string length".to_string()));
    }
    
    // Get length (first 4 bytes)
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&data[0..4]);
    let length = u32::from_be_bytes(len_bytes) as usize;
    
    // Check we have enough data
    if data.len() < 4 + length {
        return Err(AbiError::DecodingError(format!(
            "Not enough data for string: need {} bytes, got {}", 
            4 + length, data.len()
        )));
    }
    
    // Decode string
    let s = String::from_utf8_lossy(&data[4..4+length]).to_string();
    Ok((Value::String(s), 4 + length))
}

/// Decode bytes
fn decode_bytes(size: Option<usize>, data: &[u8]) -> AbiResult<(Value, usize)> {
    match size {
        Some(fixed_size) => {
            // Fixed-size bytes
            if data.len() < fixed_size {
                return Err(AbiError::DecodingError(format!(
                    "Not enough data for bytes{}: need {} bytes, got {}", 
                    fixed_size, fixed_size, data.len()
                )));
            }
            
            let bytes = data[0..fixed_size].to_vec();
            let hex = format!("0x{}", hex::encode(&bytes));
            Ok((Value::String(hex), fixed_size))
        },
        None => {
            // Dynamic bytes with length prefix
            if data.len() < 4 {
                return Err(AbiError::DecodingError("Not enough data for bytes length".to_string()));
            }
            
            // Get length (first 4 bytes)
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&data[0..4]);
            let length = u32::from_be_bytes(len_bytes) as usize;
            
            // Check we have enough data
            if data.len() < 4 + length {
                return Err(AbiError::DecodingError(format!(
                    "Not enough data for bytes: need {} bytes, got {}", 
                    4 + length, data.len()
                )));
            }
            
            let bytes = data[4..4+length].to_vec();
            let hex = format!("0x{}", hex::encode(&bytes));
            Ok((Value::String(hex), 4 + length))
        }
    }
}

/// Decode an address
fn decode_address(data: &[u8]) -> AbiResult<(Value, usize)> {
    if data.len() < 20 {
        return Err(AbiError::DecodingError(format!(
            "Not enough data for address: need 20 bytes, got {}", data.len()
        )));
    }
    
    let address = format!("0x{}", hex::encode(&data[0..20]));
    Ok((Value::String(address), 20))
}

/// Decode an array
fn decode_array(inner: &ParamType, size: Option<usize>, data: &[u8]) -> AbiResult<(Value, usize)> {
    let (length, mut offset) = match size {
        Some(fixed_size) => (fixed_size, 0),
        None => {
            // Dynamic array with length prefix
            if data.len() < 4 {
                return Err(AbiError::DecodingError("Not enough data for array length".to_string()));
            }
            
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&data[0..4]);
            let length = u32::from_be_bytes(len_bytes) as usize;
            (length, 4)
        }
    };
    
    let mut elements = Vec::with_capacity(length);
    
    for _ in 0..length {
        if offset >= data.len() {
            return Err(AbiError::DecodingError("Not enough data for array elements".to_string()));
        }
        
        let (value, bytes_read) = decode_param(inner, &data[offset..])?;
        elements.push(value);
        offset += bytes_read;
    }
    
    Ok((Value::Array(elements), offset))
}

/// Decode a hash
fn decode_hash(data: &[u8]) -> AbiResult<(Value, usize)> {
    if data.len() < 32 {
        return Err(AbiError::DecodingError(format!(
            "Not enough data for hash: need 32 bytes, got {}", data.len()
        )));
    }
    
    let hash = format!("0x{}", hex::encode(&data[0..32]));
    Ok((Value::String(hash), 32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_parse_abi() {
        let abi_json = r#"[
            {
                "type": "function",
                "name": "transfer",
                "inputs": [
                    {"name": "to", "type": "address"},
                    {"name": "amount", "type": "uint256"}
                ],
                "outputs": [
                    {"name": "success", "type": "bool"}
                ],
                "stateMutability": "nonpayable"
            },
            {
                "type": "event",
                "name": "Transfer",
                "inputs": [
                    {"name": "from", "type": "address", "indexed": true},
                    {"name": "to", "type": "address", "indexed": true},
                    {"name": "value", "type": "uint256", "indexed": false}
                ]
            }
        ]"#;
        
        let abi = ContractAbi::from_json(abi_json).unwrap();
        
        // Check functions
        assert_eq!(abi.functions.len(), 1);
        assert_eq!(abi.functions[0].name, "transfer");
        assert_eq!(abi.functions[0].inputs.len(), 2);
        assert_eq!(abi.functions[0].outputs.len(), 1);
        
        // Check events
        assert_eq!(abi.events.len(), 1);
        assert_eq!(abi.events[0].name, "Transfer");
        assert_eq!(abi.events[0].params.len(), 3);
        
        // Check we can find the function
        let transfer = abi.find_function("transfer").unwrap();
        assert_eq!(transfer.name, "transfer");
        
        // Check function selector
        let selector = abi.function_selector("transfer").unwrap();
        assert_eq!(selector.len(), 4);
    }
    
    #[test]
    fn test_encode_decode() {
        // Test encoding and decoding uint
        let value = json!(42);
        let encoded = encode_param(&ParamType::Uint(32), &value).unwrap();
        let (decoded, _) = decode_param(&ParamType::Uint(32), &encoded).unwrap();
        assert_eq!(decoded, value);
        
        // Test encoding and decoding string
        let value = json!("hello");
        let encoded = encode_param(&ParamType::String, &value).unwrap();
        let (decoded, _) = decode_param(&ParamType::String, &encoded).unwrap();
        assert_eq!(decoded, value);
        
        // Test encoding and decoding bool
        let value = json!(true);
        let encoded = encode_param(&ParamType::Bool, &value).unwrap();
        let (decoded, _) = decode_param(&ParamType::Bool, &encoded).unwrap();
        assert_eq!(decoded, value);
        
        // Test encoding and decoding address
        let value = json!("0x1234567890123456789012345678901234567890");
        let encoded = encode_param(&ParamType::Address, &value).unwrap();
        let (decoded, _) = decode_param(&ParamType::Address, &encoded).unwrap();
        assert_eq!(decoded, value);
    }
} 