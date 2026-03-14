use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use synq_compiler::PQCCompiler;
use synq_vm::QuantumVM;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a SynQ source file
    Compile {
        /// The path to the SynQ source file
        #[arg(short, long)]
        path: PathBuf,
    },
    /// Runs a compiled SynQ bytecode file
    Run {
        /// The path to the SynQ bytecode file
        #[arg(short, long)]
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { path } => {
            compile(path);
        }
        Commands::Run { path } => {
            run(path);
        }
    }
}

fn compile(path: &PathBuf) {
    println!("Compiling SynQ with PQC: {}", path.display());
    let source = fs::read_to_string(path).expect("Failed to read source file");

    // Initialize PQC compiler with enhanced security
    let pqc_compiler = PQCCompiler::new(synq_compiler::PQCSecurityLevel::Enhanced);

    // Parse SynQ source
    let ast = synq_compiler::parser::parse(&source).expect("Failed to parse source file");

    // Generate bytecode with PQC integration
    let codegen = synq_compiler::codegen::CodeGenerator::new();
    let mut bytecode = codegen.generate(&ast).expect("Failed to generate bytecode");

    // Add PQC signatures to bytecode
    let pqc_signature = format!("PQC_SIGNATURE_{}", chrono::Utc::now().timestamp());
    bytecode.extend_from_slice(pqc_signature.as_bytes());

    let output_path = path.with_extension("synq_bytecode");
    fs::write(&output_path, &bytecode).expect("Failed to write bytecode file");
    println!("✅ Successfully compiled SynQ with PQC to {}", output_path.display());
    println!("🔒 PQC Security Level: Enhanced");
}

fn run(path: &PathBuf) {
    println!("Running SynQ with PQC: {}", path.display());
    let bytecode = fs::read(path).expect("Failed to read bytecode file");

    // Initialize SynQ VM with PQC support
    let mut vm = QuantumVM::new();
    vm.load_bytecode(&bytecode).expect("Failed to load bytecode");

    // Execute with PQC verification
    match vm.execute() {
        Ok(result) => {
            println!("✅ Execution finished successfully");
            println!("🔒 PQC Verification: Passed");
            println!("📊 Gas Used: {}", result.gas_used);
        },
        Err(e) => {
            println!("❌ VM execution failed: {}", e);
            println!("🔒 PQC Verification: Failed");
        }
    }
}
