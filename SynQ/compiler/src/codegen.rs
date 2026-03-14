use crate::ast::*;
use quantumvm::{Assembler, OpCode};

pub struct CodeGenerator {
    assembler: Assembler,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            assembler: Assembler::new(),
        }
    }

    pub fn generate(mut self, ast: &[SourceUnit]) -> Result<Vec<u8>, String> {
        for item in ast {
            self.gen_source_unit(item)?;
        }
        Ok(self.assembler.build())
    }

    fn gen_source_unit(&mut self, unit: &SourceUnit) -> Result<(), String> {
        match unit {
            SourceUnit::Struct(s) => self.gen_struct(s),
            SourceUnit::Contract(c) => self.gen_contract(c),
            _ => Err("Not implemented".to_string()),
        }
    }

    fn gen_struct(&mut self, s: &StructDefinition) -> Result<(), String> {
        // For now, we don't generate any executable code for structs.
        // We would typically store metadata about the struct here.
        println!("Generating code for struct: {}", s.name);
        Ok(())
    }

    fn gen_contract(&mut self, c: &ContractDefinition) -> Result<(), String> {
        for part in &c.parts {
            match part {
                ContractPart::Function(f) => self.gen_function(f)?,
                _ => {} // Ignore other parts for now
            }
        }
        Ok(())
    }

    fn gen_function(&mut self, f: &FunctionDefinition) -> Result<(), String> {
        println!("Generating code for function: {}", f.name);
        self.assembler.emit_op(OpCode::Halt);
        Ok(())
    }
}
