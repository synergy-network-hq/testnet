//! # Synergy Network — Genesis Types
//!
//! Complete mapping of the Genesis Block Master Specification to Rust structs.
//! Each section of the spec maps directly to a struct or module here.
//!
//! ## Module Structure
//!
//! ```
//! genesis_types
//! ├── header      (Section 2)
//! ├── network     (Section 3)
//! ├── consensus   (Section 4)
//! ├── token       (Section 5)
//! ├── staking     (Section 6)
//! ├── economics   (Section 7)
//! ├── contracts   (Section 8)
//! ├── governance  (Section 9)
//! ├── bootstrap   (Section 10)
//! ├── crypto      (Section 11)
//! ├── execution   (Section 12)
//! ├── modules     (Section 13)
//! ├── upgrade     (Section 14)
//! ├── security    (Section 15)
//! └── integrity   (Section 16)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Type Aliases ─────────────────────────────────────────────────────────────

/// nano-SNRG: smallest indivisible unit (1 SNRG = 1_000_000_000 nano-SNRG)
pub type NanoSnrg = u128;

/// Unix timestamp in seconds
pub type UnixSecs = u64;

/// Bech32m-encoded address string (sYnU... or sYnQ...)
pub type Address = String;

/// Hex-encoded hash (64 hex chars = 32 bytes)
pub type HashHex = String;

/// Hex-encoded public key bytes
pub type PubKeyHex = String;

// ─────────────────────────────────────────────────────────────────────────────
// TOP-LEVEL GENESIS FILE
// Maps to: full genesis.json
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisFile {
    pub schema_version: String,         // e.g. "v1"
    pub env: NetworkType,

    // Section 2
    pub header: GenesisHeader,

    // Section 3
    pub network: NetworkIdentity,

    // Section 4
    pub consensus: ConsensusConfig,

    // Section 4.2 — validator set lives here (not inside consensus, for ergonomics)
    pub validators: Vec<GenesisValidator>,

    // Section 5
    pub token: TokenDefinition,
    pub allocations: Vec<GenesisAllocation>,
    pub vesting: Vec<VestingSchedule>,

    // Section 7
    pub economics: EconomicParams,

    // Section 8
    pub contracts: SystemContracts,
    pub precompiles: Vec<Precompile>,

    // Section 9
    pub governance: GovernanceConfig,

    // Section 10
    pub bootstrap: BootstrapConfig,

    // Section 11
    pub crypto: CryptoPolicy,

    // Section 12
    pub execution: ExecutionConfig,

    // Section 13
    pub modules: ModuleInitState,

    // Section 14
    pub upgrade: UpgradePolicy,

    // Section 15
    pub security: SecurityConfig,

    // Section 16
    pub integrity: GenesisIntegrity,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — GENESIS BLOCK HEADER
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level genesis block header (Section 2.1–2.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisHeader {
    /// Always 0 for genesis (Section 2.1)
    pub block_height: u64,

    /// 32-byte null hash — no parent (Section 2.1)
    pub parent_hash: HashHex,

    /// ISO 8601 UTC fixed timestamp string (Section 2.1)
    pub timestamp: String,

    /// Protocol identifier embedded in block (Section 2.1)
    pub extra_data: String,

    /// State commitments (Section 2.2)
    pub state_root: HashHex,
    pub transactions_root: HashHex,
    pub receipts_root: HashHex,
    pub data_root: HashHex,

    /// Consensus initialization fields (Section 2.3)
    pub consensus_fields: ConsensusHeaderFields,
}

/// Section 2.3 — Consensus header fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusHeaderFields {
    pub engine_id: String,          // e.g. "posy/v1"
    pub proposer: Option<Address>,  // None at genesis
    pub seal: Option<Vec<u8>>,      // None at genesis
    pub round: u64,                 // 0
    pub epoch: u64,                 // 0
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — NETWORK IDENTITY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Devnet,
    Testnet,
    Mainnet,
}

/// Section 3.1–3.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIdentity {
    pub chain_id: u64,              // Globally unique (Section 3.1)
    pub network_id: u64,            // May equal chain_id
    pub protocol_name: String,      // "Synergy Network"
    pub network_type: NetworkType,

    /// Section 3.2 — versioning
    pub genesis_schema_version: String,
    pub protocol_version: String,
    pub consensus_version: String,

    /// Section 3.3 — address system
    pub address_config: AddressConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressConfig {
    pub encoding: String,           // "bech32m"
    pub prefix_contract: String,    // "sYnQ"
    pub prefix_user: String,        // "sYnU"
    pub payload_length_bytes: u8,   // 20
    pub derivation_method: String,  // "SHA3-256(compressed_pubkey)[0..20]"
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — CONSENSUS CONFIGURATION (PoSy)
// ─────────────────────────────────────────────────────────────────────────────

/// Section 4.1–4.9
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub algorithm: String,          // "posy"
    pub model: String,              // "hybrid_deterministic_bft"

    // Section 4.3
    pub min_validator_count: u32,
    pub min_quorum_threshold: f64,  // e.g. 0.667
    pub min_stake_nanosnrg: NanoSnrg,

    // Section 4.4
    pub target_block_time_ms: u64,
    pub proposal_mechanism: String,
    pub leader_selection: String,

    // Section 4.5
    pub finality: FinalityConfig,

    // Section 4.6
    pub epoch: EpochConfig,

    // Section 4.7
    pub timeouts: TimeoutConfig,

    // Section 4.8
    pub slashing: SlashingConfig,

    // Section 4.9
    pub unbonding: UnbondingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityConfig {
    pub finality_type: String,          // "deterministic"
    pub confirmation_depth: u32,        // 1
    pub checkpoint_frequency_blocks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochConfig {
    pub length_blocks: u64,
    pub validator_rotation_enabled: bool,
    pub reward_distribution_interval_epochs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub proposal_ms: u64,
    pub validation_ms: u64,
    pub round_ms: u64,
    pub view_change_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingConfig {
    pub double_sign_slash_pct: f64,
    pub downtime_slash_pct: f64,
    pub invalid_block_slash_pct: f64,
    pub downtime_missed_blocks_threshold: u64,
    pub jail_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondingConfig {
    pub unbonding_period_seconds: u64,
    pub withdrawal_delay_seconds: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4.2 — GENESIS VALIDATOR
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidatorStatus {
    Active,
    Inactive,
    Jailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisValidator {
    pub operator_address: Address,
    pub consensus_pubkey: PubKeyHex,
    pub consensus_key_type: String,     // "ml-dsa" | "slh-dsa"
    pub peer_id: String,                // libp2p Peer ID
    pub stake_nanosnrg: NanoSnrg,
    pub voting_power: u64,
    pub status: ValidatorStatus,
    pub reward_address: Address,
    pub commission_rate: f64,           // 0.0–1.0
    pub moniker: String,
    pub website: Option<String>,
    pub identity: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — TOKENOMICS
// ─────────────────────────────────────────────────────────────────────────────

/// Section 5.1–5.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDefinition {
    pub name: String,               // "Synergy Token"
    pub symbol: String,             // "SNRG"
    pub decimals: u8,               // 9
    pub smallest_unit: String,      // "nano-SNRG"
    pub total_supply_cap_nanosnrg: NanoSnrg,
    pub initial_circulating_nanosnrg: NanoSnrg,
    pub minting_policy: String,     // "inflation_module_only"
}

/// Section 5.3 allocation categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AllocationCategory {
    FoundationTreasury,
    EcosystemFund,
    ValidatorIncentives,
    Team,
    Investor,
    Presale,
    LiquidityReserve,
    GrantsPool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockType {
    None,
    GovernanceLocked,
    StakingOnly,
}

/// Section 5.3 — single allocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAllocation {
    pub address: Address,
    pub amount_nanosnrg: NanoSnrg,
    pub category: AllocationCategory,
    pub locked: bool,
    pub lock_type: LockType,
}

/// Section 5.4 — vesting schedule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnlockScheduleType {
    Linear,
    Stepped,
    Milestone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    pub beneficiary_address: Address,
    pub total_allocation_nanosnrg: NanoSnrg,
    pub start_time_unix: UnixSecs,
    pub cliff_duration_seconds: u64,
    pub vesting_duration_seconds: u64,
    pub unlock_schedule: UnlockScheduleType,
    /// For stepped schedules: list of (unlock_time_offset_secs, amount_nanosnrg)
    pub steps: Option<Vec<VestingStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingStep {
    pub offset_seconds: u64,
    pub amount_nanosnrg: NanoSnrg,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — VALIDATOR STAKING STATE
// ─────────────────────────────────────────────────────────────────────────────

/// Section 6.2 — delegation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisDelegation {
    pub delegator_address: Address,
    pub validator_address: Address,
    pub amount_nanosnrg: NanoSnrg,
    pub timestamp: UnixSecs,
}

/// Section 6.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingParams {
    pub min_stake_nanosnrg: NanoSnrg,
    pub max_stake_nanosnrg: Option<NanoSnrg>,
    pub min_delegation_nanosnrg: NanoSnrg,
    pub max_delegation_nanosnrg: Option<NanoSnrg>,
}

/// Section 6.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistributionConfig {
    pub reward_pool_initial_nanosnrg: NanoSnrg,
    pub validator_share_pct: f64,
    pub delegator_share_pct: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 7 — ECONOMIC PARAMETERS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicParams {
    pub fee: FeeConfig,
    pub inflation: InflationConfig,
    pub rewards: RewardConfig,
    pub treasury: TreasuryConfig,
}

/// Section 7.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    pub base_fee_nanosnrg_per_gas: u64,
    pub min_gas_price_nanosnrg: u64,
    pub dynamic_fee_enabled: bool,
    pub fee_burn_pct: f64,
    pub validator_fee_share_pct: f64,
    pub treasury_fee_share_pct: f64,
}

/// Section 7.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationConfig {
    pub annual_rate_pct: f64,
    pub adjustment_enabled: bool,
    pub target_staking_participation: f64,
}

/// Section 7.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    pub block_reward_nanosnrg: NanoSnrg,
    pub epoch_reward_pool_nanosnrg: NanoSnrg,
    pub distribution: String,       // "stake_weighted"
}

/// Section 7.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryConfig {
    pub address: Address,
    pub control_mechanism: String,  // "governance_proposal"
    pub spending_rules: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 8 — SYSTEM CONTRACTS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContracts {
    pub staking:            ContractDef<StakingContractParams>,
    pub governance:         ContractDef<GovernanceContractParams>,
    pub treasury:           ContractDef<TreasuryContractParams>,
    pub validator_registry: ContractDef<ValidatorRegistryParams>,
    pub reward_distributor: ContractDef<RewardDistributorParams>,
    pub slashing:           ContractDef<SlashingContractParams>,
}

/// Generic contract definition wrapper (Section 8.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDef<T: Serialize> {
    pub address: Address,
    pub bytecode_hash: HashHex,
    pub init_params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingContractParams {
    pub min_stake_nanosnrg: NanoSnrg,
    pub max_stake_nanosnrg: Option<NanoSnrg>,
    pub unbonding_period_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceContractParams {
    pub quorum_pct: f64,
    pub approval_pct: f64,
    pub veto_pct: f64,
    pub min_deposit_nanosnrg: NanoSnrg,
    pub voting_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryContractParams {
    pub initial_balance_nanosnrg: NanoSnrg,
    pub required_signers: u32,
    pub signers: Vec<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRegistryParams {
    pub genesis_validator_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistributorParams {
    pub pool_address: Address,
    pub initial_pool_balance_nanosnrg: NanoSnrg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingContractParams {
    pub double_sign_slash_pct: f64,
    pub jail_duration_seconds: u64,
}

/// Section 8.3 — precompile entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precompile {
    pub address: String,    // 0x000...00N format
    pub name: String,
    pub gas_cost: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 9 — GOVERNANCE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub enabled_at_genesis: bool,
    pub governance_type: String,
    pub quorum_pct: f64,
    pub approval_pct: f64,
    pub veto_pct: f64,
    pub proposal_types: Vec<String>,
    pub min_deposit_nanosnrg: NanoSnrg,
    pub voting_duration_seconds: u64,
    pub timelock_delay_seconds: u64,
    pub execution_authority: String,
    pub emergency: EmergencyGovernanceConfig,
}

/// Section 9.5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyGovernanceConfig {
    pub guardian_addresses: Vec<Address>,
    pub guardian_threshold: u32,
    pub pause_scope: Vec<String>,
    pub auto_expire_blocks: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 10 — NETWORK BOOTSTRAP
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub bootnodes: Vec<BootnodeConfig>,
    pub dns_seeds: Vec<String>,
    pub static_peers: Vec<String>,
    pub peer_exchange: bool,
    pub ports: PortConfig,
    pub transport: TransportConfig,
}

/// Section 10.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootnodeConfig {
    pub peer_id: String,
    pub pubkey: PubKeyHex,
    pub address: String,
    pub port: u16,
    pub multiaddr: String,
}

/// Section 10.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    pub p2p: u16,
    pub rpc_http: u16,
    pub rpc_https: u16,
    pub websocket: u16,
    pub metrics: u16,
}

/// Section 10.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub tcp: bool,
    pub quic: bool,
    pub websocket: bool,
    pub encryption: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 11 — CRYPTOGRAPHY POLICY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoPolicy {
    pub signature_algorithms: Vec<String>,
    pub legacy_ecdsa_supported: bool,
    pub key_types: KeyTypeMap,
    pub hash_functions: HashFunctionConfig,
    pub domain_separation: DomainSeparationConfig,
}

/// Section 11.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyTypeMap {
    pub validator: String,      // "ml-dsa"
    pub transaction: String,    // "slh-dsa"
    pub governance: String,     // "slh-dsa"
}

/// Section 11.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashFunctionConfig {
    pub block: String,          // "sha3-256"
    pub transaction: String,    // "sha3-256"
    pub state_trie: String,     // "blake3"
}

/// Section 11.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSeparationConfig {
    pub transaction: String,    // "synergy/tx/v1/"
    pub consensus: String,      // "synergy/consensus/v1/"
    pub governance: String,     // "synergy/gov/v1/"
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 12 — EXECUTION CONFIG
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub serialization: String,          // "canonical_cbor"
    pub tx_version: u32,
    pub max_block_size_bytes: u64,
    pub max_gas_per_block: u64,
    pub vm: VmConfig,
    pub gas_model: GasModelConfig,
}

/// Section 12.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub runtime: String,                // "synqvm"
    pub max_call_depth: u32,
    pub max_memory_bytes: u64,
    pub max_contract_size_bytes: u64,
}

/// Section 12.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasModelConfig {
    pub base_tx_cost_nanosnrg: NanoSnrg,
    pub execution_cost_per_gas: u64,
    pub storage_cost_per_byte_per_block: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 13 — MODULE INITIALIZATION
// ─────────────────────────────────────────────────────────────────────────────

/// All module states at genesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInitState {
    pub staking:    StakingModuleState,
    pub governance: GovernanceModuleState,
    pub treasury:   TreasuryModuleState,
    pub rewards:    RewardsModuleState,
    pub slashing:   SlashingModuleState,
    pub identity:   IdentityModuleState,
    pub uma:        UmaModuleState,
    pub sxcp:       SxcpModuleState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingModuleState {
    pub initialized: bool,
    pub delegations_enabled: bool,
    pub delegations: Vec<GenesisDelegation>,
    pub params: StakingParams,
    pub reward_distribution: RewardDistributionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceModuleState {
    pub initialized: bool,
    pub proposals: Vec<()>,     // empty at genesis
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryModuleState {
    pub initialized: bool,
    pub treasury_address: Address,
    pub initial_balance_nanosnrg: NanoSnrg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsModuleState {
    pub initialized: bool,
    pub pool_balance_nanosnrg: NanoSnrg,
    pub block_reward_nanosnrg: NanoSnrg,
    pub epoch_reward_nanosnrg: NanoSnrg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingModuleState {
    pub initialized: bool,
    pub evidence_queue: Vec<()>,  // empty at genesis
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityModuleState {
    pub initialized: bool,
    pub contract_address: Option<Address>,
    pub reserved_names: Vec<String>,
    pub registration_fee_nanosnrg: NanoSnrg,
}

/// Section 13.7 — Universal Meta-Account module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmaModuleState {
    pub initialized: bool,
    pub session_key_ttl_seconds: u64,
    pub account_abstraction_enabled: bool,
}

/// Section 13.8 — Cross-chain (SXCP) module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SxcpModuleState {
    pub initialized: bool,
    pub enabled_at_genesis: bool,
    pub bridge_contract_address: Option<Address>,
    pub supported_chains: Vec<SxcpChainEntry>,
    pub guardian_set: Vec<Address>,
    pub rate_limit_per_block_nanosnrg: NanoSnrg,
    pub rate_limit_per_day_nanosnrg: NanoSnrg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SxcpChainEntry {
    pub chain_id: u64,
    pub chain_name: String,
    pub bridge_contract_remote: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 14 — UPGRADE POLICY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradePolicy {
    pub authority: String,
    pub signaling_threshold_pct: f64,
    pub mechanism: String,          // "on_chain_scheduled"
    pub fork_triggers: Vec<String>, // ["block_height", "timestamp"]
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 15 — SECURITY CONTROLS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub emergency_pause: EmergencyPauseConfig,
    pub rate_limits: RateLimitConfig,
    pub mempool: MempoolConfig,
    pub access_control: AccessControlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyPauseConfig {
    pub enabled: bool,
    pub guardian_multisig: Address,
    pub auto_expire_blocks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_tx_per_block_per_account: u32,
    pub bridge_max_volume_per_block_nanosnrg: NanoSnrg,
    pub bridge_max_volume_per_day_nanosnrg: NanoSnrg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    pub max_pending_per_account: u32,
    pub max_pending_global: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    pub admin_roles: String,        // "governance_contract_only"
    pub privileged_eoa: bool,       // must be false
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 16 — GENESIS RELEASE INTEGRITY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisIntegrity {
    /// Computed last — SHA3-256 of canonical genesis bytes
    pub genesis_hash: HashHex,
    pub state_root: HashHex,
    pub allocation_hash: HashHex,
    pub validator_hash: HashHex,
    pub contract_hash: HashHex,
    pub signed_by: Vec<GenesisSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisSignature {
    pub signer_address: Address,
    /// ML-DSA signature over genesis_hash
    pub signature: String,
    pub key_type: String,           // "ml-dsa"
}

// ─────────────────────────────────────────────────────────────────────────────
// VALIDATION HELPERS
// ─────────────────────────────────────────────────────────────────────────────

impl GenesisFile {
    /// Validates Section 17 invariants — call before any genesis deployment.
    /// Returns Vec of error strings (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Allocation sum check
        let alloc_sum: NanoSnrg = self.allocations.iter().map(|a| a.amount_nanosnrg).sum();
        let vest_sum: NanoSnrg  = self.vesting.iter().map(|v| v.total_allocation_nanosnrg).sum();
        let total = alloc_sum + vest_sum;
        if total != self.token.total_supply_cap_nanosnrg {
            errors.push(format!(
                "Allocation sum mismatch: {} + {} = {} != cap {}",
                alloc_sum, vest_sum, total,
                self.token.total_supply_cap_nanosnrg
            ));
        }

        // Validator count
        if self.validators.len() < self.consensus.min_validator_count as usize {
            errors.push(format!(
                "Validator count {} < minimum {}",
                self.validators.len(), self.consensus.min_validator_count
            ));
        }

        // All validators must be active
        for v in &self.validators {
            if v.status != ValidatorStatus::Active {
                errors.push(format!("Genesis validator {} is not Active", v.operator_address));
            }
        }

        // No privileged EOA
        if self.security.access_control.privileged_eoa {
            errors.push("security.access_control.privileged_eoa must be false".into());
        }

        // SXCP disabled at genesis (safe default)
        if self.modules.sxcp.enabled_at_genesis && self.modules.sxcp.guardian_set.is_empty() {
            errors.push("SXCP enabled at genesis but guardian_set is empty".into());
        }

        // Block height must be 0
        if self.header.block_height != 0 {
            errors.push("header.block_height must be 0".into());
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_file_is_serializable() {
        // Ensure the struct tree can be serialized/deserialized (compile-time check)
        let _schema: GenesisFile; // Type-checks the full struct tree
    }
}
