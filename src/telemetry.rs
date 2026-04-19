use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::NodeConfig;
use crate::info;
use crate::rpc::rpc_server::SHARED_CHAIN;
use crate::validator::{ValidatorStatus, VALIDATOR_MANAGER};

const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

pub fn start_metrics_server(bind_address: &str, config: NodeConfig, start_time: SystemTime) {
    let listener = match TcpListener::bind(bind_address) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!(
                "Warning: failed to bind metrics listener on {}: {}",
                bind_address, err
            );
            return;
        }
    };

    info!(
        "telemetry",
        "Metrics listener bound",
        "bind_address" => bind_address.to_string()
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let config = config.clone();
                thread::spawn(move || handle_connection(stream, config, start_time));
            }
            Err(err) => {
                eprintln!("Warning: metrics listener accept failed: {}", err);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, config: NodeConfig, start_time: SystemTime) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));

    let mut buffer = [0_u8; 2048];
    let read = match stream.read(&mut buffer) {
        Ok(read) => read,
        Err(_) => return,
    };

    if read == 0 {
        return;
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let mut request_line = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = request_line.next().unwrap_or_default();
    let path = request_line.next().unwrap_or("/");

    if method != "GET" {
        let _ = write_response(
            &mut stream,
            "HTTP/1.1 405 Method Not Allowed",
            "method not allowed\n",
            "text/plain; charset=utf-8",
        );
        return;
    }

    match path {
        "/metrics" => {
            let body = render_metrics(&config, start_time);
            let _ = write_response(
                &mut stream,
                "HTTP/1.1 200 OK",
                &body,
                PROMETHEUS_CONTENT_TYPE,
            );
        }
        "/healthz" | "/" => {
            let _ = write_response(
                &mut stream,
                "HTTP/1.1 200 OK",
                "ok\n",
                "text/plain; charset=utf-8",
            );
        }
        _ => {
            let _ = write_response(
                &mut stream,
                "HTTP/1.1 404 Not Found",
                "not found\n",
                "text/plain; charset=utf-8",
            );
        }
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    content_type: &str,
) -> std::io::Result<()> {
    let response = format!(
        "{status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

fn render_metrics(config: &NodeConfig, start_time: SystemTime) -> String {
    let start_time_seconds = start_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let now_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let uptime_seconds = now_seconds.saturating_sub(start_time_seconds);

    let (chain_height, chain_blocks_total, last_block_timestamp_seconds) =
        match SHARED_CHAIN.try_lock() {
            Ok(chain) => {
                let height = chain.last().map(|block| block.block_index).unwrap_or(0);
                let block_count = chain.chain.len() as u64;
                let last_timestamp = chain.last().map(|block| block.timestamp).unwrap_or(0);
                (height, block_count, last_timestamp)
            }
            Err(_) => (0, 0, 0),
        };

    let (
        validators_total,
        validator_pending_total,
        validator_active_total,
        validator_inactive_total,
        validator_jailed_total,
        validator_slashed_total,
        clusters_total,
    ) = match VALIDATOR_MANAGER.registry.try_lock() {
        Ok(registry) => {
            let mut active = 0_u64;
            let mut inactive = 0_u64;
            let mut jailed = 0_u64;
            let mut slashed = 0_u64;
            for validator in registry.validators.values() {
                match validator.status {
                    ValidatorStatus::Active => active += 1,
                    ValidatorStatus::Inactive => inactive += 1,
                    ValidatorStatus::Jailed => jailed += 1,
                    ValidatorStatus::Slashed => slashed += 1,
                    ValidatorStatus::Pending => {}
                }
            }
            (
                registry.validators.len() as u64,
                registry.pending_registrations.len() as u64,
                active,
                inactive,
                jailed,
                slashed,
                registry.clusters.len() as u64,
            )
        }
        Err(_) => (0, 0, 0, 0, 0, 0, 0),
    };

    let configured_peer_targets = (config.network.bootnodes.len()
        + config.network.seed_servers.len()
        + config.network.bootstrap_dns_records.len()
        + config.network.additional_dial_targets.len()
        + config.network.persistent_peers.len()) as u64;

    let mut body = String::new();
    push_metric_header(
        &mut body,
        "synergy_node_info",
        "Static identity and role labels for this Synergy node.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_node_info{{network=\"{}\",role=\"{}\",node_id=\"{}\",node_name=\"{}\",validator_address=\"{}\"}} 1\n",
        escape_label_value(&config.network.name),
        escape_label_value(&config.identity.role),
        escape_label_value(&config.identity.node_id),
        escape_label_value(&config.p2p.node_name),
        escape_label_value(&config.node.validator_address),
    ));

    push_metric_header(
        &mut body,
        "synergy_chain_height",
        "Latest block height in the shared chain state.",
        "gauge",
    );
    body.push_str(&format!("synergy_chain_height {chain_height}\n"));

    push_metric_header(
        &mut body,
        "synergy_chain_blocks_total",
        "Number of blocks currently held in memory.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_chain_blocks_total {chain_blocks_total}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_chain_last_block_timestamp_seconds",
        "Unix timestamp for the latest block in the shared chain state.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_chain_last_block_timestamp_seconds {last_block_timestamp_seconds}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_validator_registry_total",
        "Number of validators tracked in the local registry.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_validator_registry_total {validators_total}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_validator_pending_total",
        "Number of validator registrations still pending approval.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_validator_pending_total {validator_pending_total}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_validator_status_total",
        "Number of validators in each status bucket.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_validator_status_total{{status=\"active\"}} {validator_active_total}\n"
    ));
    body.push_str(&format!(
        "synergy_validator_status_total{{status=\"inactive\"}} {validator_inactive_total}\n"
    ));
    body.push_str(&format!(
        "synergy_validator_status_total{{status=\"jailed\"}} {validator_jailed_total}\n"
    ));
    body.push_str(&format!(
        "synergy_validator_status_total{{status=\"slashed\"}} {validator_slashed_total}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_validator_clusters_total",
        "Number of validator clusters in the local registry.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_validator_clusters_total {clusters_total}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_configured_peer_targets_total",
        "Total configured peer bootstrap targets for this node.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_configured_peer_targets_total {configured_peer_targets}\n"
    ));

    push_metric_header(
        &mut body,
        "synergy_node_uptime_seconds",
        "Process uptime in seconds.",
        "counter",
    );
    body.push_str(&format!("synergy_node_uptime_seconds {uptime_seconds}\n"));

    push_metric_header(
        &mut body,
        "synergy_process_start_time_seconds",
        "Unix timestamp for the current process start time.",
        "gauge",
    );
    body.push_str(&format!(
        "synergy_process_start_time_seconds {start_time_seconds}\n"
    ));

    body
}

fn push_metric_header(body: &mut String, name: &str, help: &str, metric_type: &str) {
    body.push_str(&format!("# HELP {name} {help}\n"));
    body.push_str(&format!("# TYPE {name} {metric_type}\n"));
}

fn escape_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::render_metrics;
    use crate::config::NodeConfig;
    use std::env;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn set(path: &Path) -> Self {
            let previous = env::current_dir().expect("current dir should resolve");
            env::set_current_dir(path).expect("current dir should update");
            Self { previous }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.previous).expect("current dir should restore");
        }
    }

    #[test]
    fn render_metrics_includes_identity_labels() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate manifest should live under repo root");
        let _cwd = CurrentDirGuard::set(repo_root);

        let mut config = NodeConfig::default();
        config.network.name = "synergy-testnet-beta".to_string();
        config.identity.role = "validator".to_string();
        config.identity.node_id = "GenVal-01".to_string();
        config.p2p.node_name = "genesisval1".to_string();
        config.node.validator_address = "synv1test".to_string();

        let body = render_metrics(&config, SystemTime::now() - Duration::from_secs(5));

        assert!(body.contains("synergy_node_info"));
        assert!(body.contains("role=\"validator\""));
        assert!(body.contains("node_id=\"GenVal-01\""));
        assert!(body.contains("validator_address=\"synv1test\""));
    }
}
