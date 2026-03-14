use pest::Parser;
use pest::iterators::Pair;
use crate::ast::*;

#[derive(Parser)]
#[grammar = "synq.pest"]
pub struct SynQParser;

pub fn parse(source: &str) -> Result<Vec<SourceUnit>, pest::error::Error<Rule>> {
    let pairs = SynQParser::parse(Rule::source_file, source)?;
    let mut ast = vec![];

    for pair in pairs.into_iter().next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::item => {
                let item = pair.into_inner().next().unwrap();
                match item.as_rule() {
                    Rule::struct_definition => {
                        ast.push(SourceUnit::Struct(parse_struct(item)));
                    }
                    Rule::contract_definition => {
                        ast.push(SourceUnit::Contract(parse_contract(item)));
                    }
                    _ => unreachable!(),
                }
            }
            Rule::EOI => (),
            _ => {} // Ignore whitespace and comments
        }
    }

    Ok(ast)
}

fn parse_struct(pair: Pair<Rule>) -> StructDefinition {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let fields = inner.map(parse_struct_field).collect();
    StructDefinition { name, fields }
}

fn parse_struct_field(pair: Pair<Rule>) -> Parameter {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let ty = parse_type(inner.next().unwrap());
    Parameter { ty, name, is_indexed: false }
}

fn parse_contract(pair: Pair<Rule>) -> ContractDefinition {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let parts = inner.map(parse_contract_part).collect();
    ContractDefinition { name, parts }
}

fn parse_contract_part(pair: Pair<Rule>) -> ContractPart {
    let item = pair.into_inner().next().unwrap();
    match item.as_rule() {
        Rule::state_variable_declaration => {
            ContractPart::StateVariable(parse_state_variable(item))
        }
        Rule::function_definition => {
            ContractPart::Function(parse_function(item))
        }
        _ => unreachable!(),
    }
}

fn parse_state_variable(pair: Pair<Rule>) -> StateVariableDeclaration {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let ty = parse_type(inner.next().unwrap());
    let is_public = inner.next().is_some();
    StateVariableDeclaration { ty, name, is_public }
}

fn parse_function(pair: Pair<Rule>) -> FunctionDefinition {
    println!("Parsing function: {:?}", pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let params = inner
        .clone()
        .filter(|p| p.as_rule() == Rule::param)
        .map(parse_param)
        .collect();
    let _body = inner.last().unwrap(); // ignore for now
    FunctionDefinition {
        name,
        params,
        returns: None,
        body: Block { statements: vec![] },
        is_public: false,
    }
}

fn parse_param(pair: Pair<Rule>) -> Parameter {
    println!("Parsing param: {:?}", pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let ty = parse_type(inner.next().unwrap());
    Parameter { ty, name, is_indexed: false }
}


fn parse_type(pair: Pair<Rule>) -> Type {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str();
    match name {
        "Address" => Type::Address,
        "UInt256" => Type::UInt256,
        "Bool" => Type::Bool,
        "Bytes" => Type::Bytes,
        "DilithiumPublicKey" => Type::DilithiumPublicKey,
        "FalconPublicKey" => Type::FalconPublicKey,
        "KyberPublicKey" => Type::KyberPublicKey,
        "DilithiumSignature" => Type::DilithiumSignature,
        "FalconSignature" => Type::FalconSignature,
        _ => Type::Address, // Placeholder
    }
}
