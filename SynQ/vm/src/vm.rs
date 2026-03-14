use std::collections::HashMap;
use super::opcode::{OpCode, VMError};
use pqc_shims::{dilithium, kyber, falcon, sphincs};

// Value types that can be stored on the stack
#[derive(Debug, Clone)]
pub enum Value {
    I32(i32),
    I64(i64),
    Bytes(Vec<u8>),
    Bool(bool),
}

impl Value {
    pub fn as_i32(&self) -> Result<i32, VMError> {
        match self {
            Value::I32(v) => Ok(*v),
            _ => Err(VMError::RuntimeError("Expected i32".to_string())),
        }
    }

    pub fn as_i64(&self) -> Result<i64, VMError> {
        match self {
            Value::I64(v) => Ok(*v),
            Value::I32(v) => Ok(*v as i64),
            _ => Err(VMError::RuntimeError("Expected i64".to_string())),
        }
    }

    pub fn as_bytes(&self) -> Result<&[u8], VMError> {
        match self {
            Value::Bytes(v) => Ok(v),
            _ => Err(VMError::RuntimeError("Expected bytes".to_string())),
        }
    }

    pub fn as_bool(&self) -> Result<bool, VMError> {
        match self {
            Value::Bool(v) => Ok(*v),
            Value::I32(v) => Ok(*v != 0),
            _ => Err(VMError::RuntimeError("Expected bool".to_string())),
        }
    }
}

// Bytecode header
#[derive(Debug)]
pub struct Header {
    pub magic: u32,
    pub version: u8,
    pub header_length: u16,
    pub code_length: u32,
    pub data_length: u32,
}

impl Header {
    pub const MAGIC: u32 = 0x51564D00; // QVM\0

    pub fn parse(bytes: &[u8]) -> Result<Self, VMError> {
        if bytes.len() < 12 {
            return Err(VMError::InvalidBytecode("Header too short".to_string()));
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != Self::MAGIC {
            return Err(VMError::InvalidBytecode("Invalid magic number".to_string()));
        }

        let version = bytes[4];
        let header_length = u16::from_le_bytes([bytes[5], bytes[6]]);
        let code_length = u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]);
        let data_length = u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]);

        Ok(Header {
            magic,
            version,
            header_length,
            code_length,
            data_length,
        })
    }
}


// The main VM struct
pub struct QuantumVM {
    pub stack: Vec<Value>,
    memory: HashMap<usize, Value>,
    code: Vec<u8>,
    data: Vec<u8>,
    pc: usize,
    call_stack: Vec<usize>,
    halted: bool,
}

impl QuantumVM {
    pub fn new() -> Self {
        QuantumVM {
            stack: Vec::new(),
            memory: HashMap::new(),
            code: Vec::new(),
            data: Vec::new(),
            pc: 0,
            call_stack: Vec::new(),
            halted: false,
        }
    }

    pub fn load_bytecode(&mut self, bytecode: &[u8]) -> Result<(), VMError> {
        let header = Header::parse(bytecode)?;

        let header_end = header.header_length as usize;
        let code_end = header_end + header.code_length as usize;
        let data_end = code_end + header.data_length as usize;

        if bytecode.len() < data_end {
            return Err(VMError::InvalidBytecode("Bytecode too short".to_string()));
        }

        self.code = bytecode[header_end..code_end].to_vec();
        self.data = bytecode[code_end..data_end].to_vec();
        self.pc = 0;
        self.halted = false;

        Ok(())
    }

    pub fn execute(&mut self) -> Result<(), VMError> {
        while !self.halted && self.pc < self.code.len() {
            self.execute_instruction()?;
        }
        Ok(())
    }

    fn execute_instruction(&mut self) -> Result<(), VMError> {
        if self.pc >= self.code.len() {
            return Err(VMError::InvalidAddress(self.pc));
        }

        let opcode = OpCode::try_from(self.code[self.pc])?;
        self.pc += 1;

        match opcode {
            OpCode::Push => {
                let value = self.read_i32()?;
                self.push(Value::I32(value))?;
            }
            OpCode::Pop => {
                self.pop()?;
            }
            OpCode::Dup => {
                let value = self.peek()?.clone();
                self.push(value)?;
            }
            OpCode::Swap => {
                let a = self.pop()?;
                let b = self.pop()?;
                self.push(a)?;
                self.push(b)?;
            }
            OpCode::Add => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::I32(a + b))?;
            }
            OpCode::Sub => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::I32(a - b))?;
            }
            OpCode::Mul => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::I32(a * b))?;
            }
            OpCode::Div => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                if b == 0 {
                    return Err(VMError::RuntimeError("Division by zero".to_string()));
                }
                self.push(Value::I32(a / b))?;
            }
            OpCode::Eq => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::Bool(a == b))?;
            }
            OpCode::Ne => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::Bool(a != b))?;
            }
            OpCode::Lt => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::Bool(a < b))?;
            }
            OpCode::Le => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::Bool(a <= b))?;
            }
            OpCode::Gt => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::Bool(a > b))?;
            }
            OpCode::Ge => {
                let b = self.pop()?.as_i32()?;
                let a = self.pop()?.as_i32()?;
                self.push(Value::Bool(a >= b))?;
            }
            OpCode::Jump => {
                let addr = self.read_u32()? as usize;
                if addr >= self.code.len() {
                    return Err(VMError::InvalidAddress(addr));
                }
                self.pc = addr;
            }
            OpCode::JumpIf => {
                let addr = self.read_u32()? as usize;
                let condition = self.pop()?.as_bool()?;
                if condition {
                    if addr >= self.code.len() {
                        return Err(VMError::InvalidAddress(addr));
                    }
                    self.pc = addr;
                }
            }
            OpCode::Call => {
                let addr = self.read_u32()? as usize;
                if addr >= self.code.len() {
                    return Err(VMError::InvalidAddress(addr));
                }
                self.call_stack.push(self.pc);
                self.pc = addr;
            }
            OpCode::Return => {
                if let Some(return_addr) = self.call_stack.pop() {
                    self.pc = return_addr;
                } else {
                    return Err(VMError::RuntimeError("Return without call".to_string()));
                }
            }
            OpCode::Load => {
                let addr = self.pop()?.as_i32()? as usize;
                if let Some(value) = self.memory.get(&addr) {
                    self.push(value.clone())?;
                } else {
                    return Err(VMError::InvalidAddress(addr));
                }
            }
            OpCode::Store => {
                let addr = self.pop()?.as_i32()? as usize;
                let value = self.pop()?;
                self.memory.insert(addr, value);
            }
            OpCode::LoadImm => {
                let len = self.read_u32()? as usize;
                let bytes = self.read_bytes(len)?;
                self.push(Value::Bytes(bytes))?;
            }
            OpCode::DilithiumVerify => {
                let public_key = self.pop()?.as_bytes()?.to_vec();
                let message = self.pop()?.as_bytes()?.to_vec();
                let signature = self.pop()?.as_bytes()?.to_vec();

                let result = dilithium::verify(&message, &signature, &public_key);
                self.push(Value::Bool(result))?;
            }
            OpCode::KyberKeyExchange => {
                // This opcode performs decapsulation as per the spec.
                let private_key = self.pop()?.as_bytes()?.to_vec();
                let ciphertext = self.pop()?.as_bytes()?.to_vec();

                let shared_secret = kyber::decaps(&ciphertext, &private_key);
                self.push(Value::Bytes(shared_secret))?;
            }
            OpCode::FalconVerify => {
                let public_key = self.pop()?.as_bytes()?.to_vec();
                let message = self.pop()?.as_bytes()?.to_vec();
                let signature = self.pop()?.as_bytes()?.to_vec();

                let result = falcon::verify(&message, &signature, &public_key);
                self.push(Value::Bool(result))?;
            }
            OpCode::SphincsVerify => {
                let public_key = self.pop()?.as_bytes()?.to_vec();
                let message = self.pop()?.as_bytes()?.to_vec();
                let signature = self.pop()?.as_bytes()?.to_vec();

                let result = sphincs::verify(&message, &signature, &public_key);
                self.push(Value::Bool(result))?;
            }
            OpCode::Print => {
                let value = self.pop()?;
                println!("{:?}", value);
            }
            OpCode::Halt => {
                self.halted = true;
            }
        }

        Ok(())
    }

    fn push(&mut self, value: Value) -> Result<(), VMError> {
        if self.stack.len() >= 1000 {
            return Err(VMError::StackOverflow);
        }
        self.stack.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, VMError> {
        self.stack.pop().ok_or(VMError::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value, VMError> {
        self.stack.last().ok_or(VMError::StackUnderflow)
    }

    fn read_i32(&mut self) -> Result<i32, VMError> {
        if self.pc + 4 > self.code.len() {
            return Err(VMError::InvalidAddress(self.pc));
        }
        let bytes = [
            self.code[self.pc],
            self.code[self.pc + 1],
            self.code[self.pc + 2],
            self.code[self.pc + 3],
        ];
        self.pc += 4;
        Ok(i32::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, VMError> {
        if self.pc + 4 > self.code.len() {
            return Err(VMError::InvalidAddress(self.pc));
        }
        let bytes = [
            self.code[self.pc],
            self.code[self.pc + 1],
            self.code[self.pc + 2],
            self.code[self.pc + 3],
        ];
        self.pc += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, VMError> {
        if self.pc + len > self.code.len() {
            return Err(VMError::InvalidAddress(self.pc));
        }
        let bytes = self.code[self.pc..self.pc + len].to_vec();
        self.pc += len;
        Ok(bytes)
    }
}
