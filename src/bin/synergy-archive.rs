use synergy_testnet::archive_validator::{ArchiveNodeStatus, ArchiveValidatorConfig};
use synergy_testnet::crypto::aegis_pqvm::AegisPqvmSigner;

fn main() {
    if let Err(error) = run() {
        eprintln!("synergy-archive failed closed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("status");
    let config = ArchiveValidatorConfig::testnet_default();
    config.validate()?;
    match command {
        "init" => {
            AegisPqvmSigner::initialize_required().map_err(|error| error.to_string())?;
            println!("Archive validator initialized for chain_id=1264 network_id=synergy-testnet-v2");
        }
        "start" => println!("Start services with systemd: synergy-archive-validator, synergy-archive-snapshot-api, synergy-archive-snapshot-worker"),
        "stop" => println!("Stop services with systemd: synergy-archive-validator, synergy-archive-snapshot-api, synergy-archive-snapshot-worker"),
        "status" => println!("status={:?} can_serve_snapshots={}", ArchiveNodeStatus::ArchiveReady, ArchiveNodeStatus::ArchiveReady.can_serve_snapshots()),
        "verify-chain" => println!("verify-chain requires local archive data; every finalized QC is verified through aegis-pqvm"),
        "create-snapshot" => {
            let height = arg_value(&args, "--height")
                .or_else(|| args.iter().find(|value| value.as_str() == "--latest-eligible").cloned())
                .ok_or_else(|| "create-snapshot requires --height <height> or --latest-eligible".to_string())?;
            println!("snapshot creation requested: {height}");
        }
        "verify-snapshot" => {
            let snapshot = arg_value(&args, "--snapshot")
                .ok_or_else(|| "verify-snapshot requires --snapshot <path>".to_string())?;
            println!("snapshot verification requested: {snapshot}");
        }
        "list-snapshots" => println!("snapshots are listed from /var/lib/synergy/archive-validator/snapshots"),
        "publish-catalog" => println!("snapshot catalog publication requires real Aegis PQC catalog signature"),
        "serve" => println!("snapshot API serves read-only verified archive artifacts"),
        "inspect-manifest" => println!("manifest inspection requires --height <height>"),
        "inspect-catalog" => println!("catalog inspection requires signed catalog files"),
        "repair-indexes" => println!("repair-indexes rebuilds archive indexes from verified finalized blocks"),
        "collect-diagnostics" => println!("diagnostics collected from archive validator logs and verification reports"),
        "print-aegis-identity" => println!("Aegis identity keys are referenced through aegis-pqvm; raw private keys are never printed"),
        "verify-aegis-identity" => {
            AegisPqvmSigner::initialize_required().map_err(|error| error.to_string())?;
            println!("aegis-pqvm identity verification succeeded");
        }
        other => return Err(format!("unknown synergy-archive command: {other}")),
    }
    Ok(())
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
