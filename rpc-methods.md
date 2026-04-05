# Synergy Testnet-Beta RPC Methods

This document lists all available JSON-RPC methods for the Synergy Testnet-Beta.

## Endpoint

- **URL**: `http://localhost:8545` (default RPC port)
- **Protocol**: JSON-RPC 2.0
- **Content-Type**: `application/json`

## Request Format

```json
{
  "jsonrpc": "2.0",
  "method": "method_name",
  "params": [...],
  "id": 1
}
```

## Response Format

```json
{
  "jsonrpc": "2.0",
  "result": {...},
  "id": 1
}
```

## Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error message"
  },
  "id": 1
}
```

---

## Blockchain Queries

### `synergy_blockNumber`
Get the current block number (height).

**Parameters**: None

**Returns**: `number` - Current block number

---

### `synergy_getBlockNumber`
Get the current block number (height). (Alias for `synergy_blockNumber`)

**Parameters**: None

**Returns**: `number` - Current block number

---

### `synergy_getBlockByNumber`
Get a block by its block number.

**Parameters**:
- `blockNumber` (number) - The block number to fetch

**Returns**: Block object or `null` if not found

**Block Object**:
```json
{
  "block_index": 123,
  "timestamp": 1640995200,
  "hash": "...",
  "previous_hash": "...",
  "parent_hash": "...",
  "validator_id": "synv...",
  "validator": "synv...",
  "nonce": 0,
  "tx_count": 10,
  "transactions": [...]
}
```

---

### `synergy_getBlockByHash`
Get a block by its hash.

**Parameters**:
- `blockHash` (string) - The block hash to fetch

**Returns**: Block object or `null` if not found

---

### `synergy_getLatestBlock`
Get the most recent block.

**Parameters**: None

**Returns**: Block object or `null` if no blocks exist

---

### `synergy_getBlockRange`
Get a range of blocks.

**Parameters**:
- `start` (number) - Starting block number
- `end` (number) - Ending block number

**Returns**: Array of block objects

---

### `synergy_getDeterminismDigest`
Get a comprehensive determinism digest for verifying state consistency across nodes.

**Parameters**: None

**Returns**: Object with:
- `block_height`: Current block height
- `block_hash`: Current block hash
- `state_root`: Combined state root hash
- `receipt_hash`: Transaction receipt hash
- `token_state_hash`: Token state hash
- `validator_registry_hash`: Validator registry hash
- `chain_state_hash`: Chain state hash

---

## Transaction Methods

### `synergy_sendTransaction`
Submit a signed transaction to the network.

**Parameters**:
- `transaction` (object) - Transaction object with:
  - `sender`: Sender address
  - `receiver`: Receiver address
  - `amount`: Amount in nWei
  - `nonce`: Transaction nonce
  - `gas_price`: Gas price
  - `gas_limit`: Gas limit
  - `data`: Optional transaction data (hex)
  - `signature`: Transaction signature (hex)
  - `signature_algorithm`: Signature algorithm used

**Returns**: 
```json
{
  "success": true,
  "tx_hash": "syntxn-...",
  "message": "Transaction submitted"
}
```
or error object with `error` field

---

### `synergy_getTransactionByHash`
Get a transaction by its hash. Supports multiple hash formats:
- Full hash: `syntxn-...`
- Raw hash: `a0d53ef9...`
- With or without `0x` prefix

**Parameters**:
- `txHash` (string) - Transaction hash

**Returns**: Transaction object or `null` if not found

**Transaction Object**:
```json
{
  "hash": "syntxn-...",
  "sender": "synw...",
  "receiver": "synw...",
  "from": "synw...",
  "to": "synw...",
  "amount": 1000000000,
  "amount_snrg": 1.0,
  "nonce": 1,
  "gas_price": 1000,
  "gas_limit": 21000,
  "fee": 21000000,
  "timestamp": 1640995200,
  "data": "...",
  "signature_algorithm": "fndsa",
  "signature": "...",
  "status": "confirmed",
  "block_number": 123,
  "transaction_index": 0
}
```

---

### `synergy_getTransactionPool`
Get all pending transactions in the transaction pool.

**Parameters**: None

**Returns**: Array of pending transaction objects

---

### `synergy_getTransactionsInBlock`
Get all transactions in a specific block.

**Parameters**:
- `blockNumber` (number) - Block number

**Returns**: Array of transaction objects

---

## Node Information

### `synergy_nodeInfo`
Get node information.

**Parameters**: None

**Returns**: Node info object with:
- `name`: Node name
- `version`: Node version
- `protocolVersion`: Protocol version (null if not set)
- `networkId`: Network ID (338639 for testnet-beta)
- `chainId`: Chain ID (338639 for testnet-beta)
- `consensus`: Consensus algorithm
- `syncing`: Sync status (boolean)
- `currentBlock`: Current block number
- `timestamp`: Current timestamp

---

### `synergy_getNodeStatus`
Get detailed node status including average block time and peer count.

**Parameters**: None

**Returns**: Status object with:
- `node_type`: Node type (null if not set)
- `status`: Node status ("running")
- `uptime`: Uptime percentage (string, e.g., "99.9%")
- `uptime_seconds`: Uptime in seconds
- `version`: Node version
- `network`: Network name
- `sync_status`: Sync status ("synced" or "syncing")
- `last_block`: Last block number
- `avg_block_time`: Average block time in seconds
- `average_block_time`: Alias for avg_block_time
- `peers_connected`: Number of connected peers
- `peer_count`: Alias for peers_connected
- `peers`: Alias for peers_connected
- `timestamp`: Current timestamp

---

### `synergy_getSyncStatus`
Get detailed synchronization status.

**Parameters**: None

**Returns**: Object with:
- `syncing`: Whether node is currently syncing
- `current_block`: Current block height
- `highest_block`: Highest known block in network
- `starting_block`: Block height when sync started
- `sync_percentage`: Sync progress percentage
- `state`: Current sync state ("Idle", "Discovering", "Downloading", "Validating", "Applying", "Synced")

---

### `synergy_status`
Simple status check (legacy method).

**Parameters**: None

**Returns**: `"ok"` string

---

## Validator Methods

### `synergy_getValidators`
Get all active validators.

**Parameters**: None

**Returns**: Array of validator objects with:
- `address`: Validator address
- `public_key`: Validator public key
- `name`: Validator name
- `stake_amount`: Staked amount
- `synergy_score`: Current synergy score
- `uptime_percentage`: Uptime percentage
- `total_blocks_produced`: Total blocks produced
- `cluster_id`: Cluster ID
- `last_active`: Last activity timestamp
- `status`: Validator status

---

### `synergy_getValidator`
Get a specific validator by address.

**Parameters**:
- `address` (string) - Validator address

**Returns**: Validator object or `null` if not found

---

### `synergy_getTopValidators`
Get top validators by rank.

**Parameters**:
- `count` (number, optional) - Number of validators to return (default: 10)

**Returns**: Array of top validator objects

---

### `synergy_registerValidator`
Register a new validator.

**Parameters**:
- `address` (string) - Validator address
- `public_key` (string) - Validator public key
- `name` (string) - Validator name
- `stake_amount` (number) - Initial stake amount

**Returns**: 
```json
{
  "success": true,
  "message": "Validator registered successfully"
}
```
or error object

---

### `synergy_approveValidator`
Approve a validator registration.

**Parameters**:
- `address` (string) - Validator address

**Returns**: 
```json
{
  "success": true,
  "message": "Validator approved successfully"
}
```
or error object

---

### `synergy_slashValidator`
Slash a validator (penalize for misbehavior).

**Parameters**:
- `address` (string) - Validator address
- `reason` (string) - Reason for slashing

**Returns**: 
```json
{
  "success": true,
  "message": "Validator slashed successfully"
}
```
or error object

---

### `synergy_getValidatorStats`
Get validator statistics.

**Parameters**: None

**Returns**: Stats object with:
- `total_validators`: Total number of validators
- `active_validators`: Array of active validators
- `top_validators`: Array of top validators
- `epoch_rewards`: Epoch rewards information

---

### `synergy_getValidatorActivity`
Get validator activity information.

**Parameters**: None

**Returns**: Activity object with:
- `validators`: Array of validator activity objects
- `total_active`: Total active validators
- `average_synergy_score`: Average synergy score

**Validator Activity Object**:
```json
{
  "address": "synv...",
  "name": "Validator Name",
  "synergy_score": 0.95,
  "blocks_produced": 100,
  "uptime": "99.9%",
  "cluster_id": 1,
  "stake_amount": 1000000,
  "last_active": 1640995200
}
```

---

### `synergy_getSynergyScoreBreakdown`
Get detailed Synergy Score components for a validator.

**Parameters**:
- `address` (string) - Validator address

**Returns**: Object with:
- `address`: Validator address
- `total_score`: Current normalized Synergy Score
- `components`: Synergy score component object containing:
  - `stake_weight`: Stake weight component
  - `reputation`: Reputation component
  - `contribution_index`: Contribution index
  - `cartelization_penalty`: Cartelization penalty
  - `normalized_score`: Normalized score
  - `last_updated`: Last update timestamp

---

### `synergy_getBlockValidationStatus`
Get block validation status.

**Parameters**: None

**Returns**: Validation status object with:
- `current_block_height`: Current block height
- `recent_blocks`: Array of recent block validation info
- `validation_queue`: Pending validation queue (empty array)
- `active_validators`: Number of active validators
- `total_validators`: Total number of validators
- `cluster_info`: Cluster information with:
  - `active_clusters`: Number of active clusters
  - `total_stake`: Total stake across clusters

---

## Token Methods

### `synergy_getTokenBalance`
Get token balance for an address.

**Parameters**:
- `address` (string) - Address to check
- `token` (string) - Token symbol (e.g., "SNRG")

**Returns**: Balance in nWei (nano-wei, smallest unit)

**Note**: 1 SNRG = 1,000,000,000 nWei

---

### `synergy_getTokens`
Get all tokens on the network.

**Parameters**: None

**Returns**: Array of token objects with:
- `symbol`: Token symbol
- `name`: Token name
- `decimals`: Number of decimals
- `total_supply`: Total supply
- `max_supply`: Maximum supply
- `mintable`: Whether token is mintable
- `burnable`: Whether token is burnable
- `created_at`: Creation timestamp
- `creator`: Creator address

---

### `synergy_getAllBalances`
Get all token balances for an address.

**Parameters**:
- `address` (string) - Address to check

**Returns**: Object mapping token symbols to balances (in nWei)

---

### `synergy_sendTokens`
Send tokens from one address to another.

**Parameters**:
- `from` (string) - Sender address
- `to` (string) - Recipient address
- `token_symbol` (string) - Token symbol (e.g., "SNRG")
- `amount` (number) - **Amount in SNRG** (will be converted to nWei internally)

**Returns**: 
```json
{
  "success": true,
  "tx_hash": "syntxn-...",
  "transaction": {...},
  "message": "Transaction submitted"
}
```
or error object

**Note**: The `amount` parameter should be specified in SNRG units. The RPC automatically converts it to nWei (1 SNRG = 1,000,000,000 nWei).

---

### `synergy_createToken`
Create a new token.

**Parameters**:
- `symbol` (string) - Token symbol
- `name` (string) - Token name
- `decimals` (number) - Number of decimals
- `total_supply` (number) - Total supply
- `creator` (string) - Creator address

**Returns**: 
```json
{
  "success": true,
  "message": "Token created successfully"
}
```
or error object

---

### `synergy_mintTokens`
Mint new tokens.

**Parameters**:
- `to` (string) - Recipient address
- `token_symbol` (string) - Token symbol
- `amount` (number) - Amount to mint

**Returns**: 
```json
{
  "success": true,
  "message": "Tokens minted successfully"
}
```
or error object

---

### `synergy_burnTokens`
Burn tokens.

**Parameters**:
- `from` (string) - Address to burn from
- `token_symbol` (string) - Token symbol
- `amount` (number) - Amount to burn

**Returns**: 
```json
{
  "success": true,
  "message": "Tokens burned successfully"
}
```
or error object

---

### `synergy_transferTokens`
Transfer tokens (low-level method).

**Parameters**:
- `from` (string) - Sender address
- `to` (string) - Recipient address
- `token_symbol` (string) - Token symbol
- `amount` (number) - Amount to transfer

**Returns**: 
```json
{
  "success": true,
  "message": "Tokens transferred successfully"
}
```
or error object

---

### `synergy_getTokenStats`
Get token statistics.

**Parameters**: None

**Returns**: Array of token stat objects with:
- `symbol`: Token symbol
- `name`: Token name
- `total_supply`: Total supply
- `total_staked`: Total staked amount
- `holders`: Number of holders

---

### `synergy_getTransferHistory`
Get transfer history for an address.

**Parameters**:
- `address` (string) - Address to check
- `limit` (number, optional) - Maximum number of transfers to return (default: 50)

**Returns**: Array of transfer objects with:
- `from`: Sender address
- `to`: Recipient address
- `token_symbol`: Token symbol
- `amount`: Amount transferred
- `fee`: Transaction fee
- `timestamp`: Transfer timestamp
- `tx_hash`: Transaction hash
- `block_height`: Block height

---

## Staking Methods

### `synergy_stakeTokens`
Create a staking transaction.

**Parameters**:
- `staker` (string) - Staker address
- `validator` (string) - Validator address
- `token_symbol` (string) - Token symbol
- `amount` (number) - **Amount in SNRG** (will be converted to nWei)

**Returns**: 
```json
{
  "success": true,
  "transaction": {...},
  "message": "Staking transaction created successfully"
}
```
or error object

---

### `synergy_stakeTokensDirect`
Stake tokens directly (without creating a transaction).

**Parameters**:
- `staker` (string) - Staker address
- `validator` (string) - Validator address
- `token_symbol` (string) - Token symbol
- `amount` (number) - **Amount in SNRG** (will be converted to nWei)

**Returns**: 
```json
{
  "success": true,
  "message": "Tokens staked successfully"
}
```
or error object

---

### `synergy_unstakeTokens`
Unstake tokens.

**Parameters**:
- `staker` (string) - Staker address
- `validator` (string) - Validator address
- `token_symbol` (string) - Token symbol
- `amount` (number) - Amount to unstake

**Returns**: 
```json
{
  "success": true,
  "message": "Tokens unstaked successfully"
}
```
or error object

---

### `synergy_getStakedBalance`
Get staked balance for an address.

**Parameters**:
- `address` (string) - Address to check
- `token_symbol` (string) - Token symbol

**Returns**: 
```json
{
  "balance": 1000000
}
```

---

### `synergy_getStakingInfo`
Get staking information for an address.

**Parameters**:
- `address` (string) - Address to check

**Returns**: Array of staking info objects with:
- `validator`: Validator address
- `staked_amount`: Staked amount
- `rewards_earned`: Rewards earned
- `staking_timestamp`: Staking timestamp

---

## Wallet Methods

### `synergy_createWallet`
Create a new wallet.

**Parameters**: None

**Returns**: 
```json
{
  "address": "synw...",
  "message": "Wallet created successfully"
}
```
or error object

---

### `synergy_getWallet`
Get wallet information.

**Parameters**:
- `address` (string) - Wallet address

**Returns**: Wallet object or `null` if not found

**Wallet Object**:
```json
{
  "address": "synw...",
  "public_key": "...",
  "balances": {
    "SNRG": 1000000
  },
  "nonce": 0
}
```

---

### `synergy_createWalletFromKeypair`
Create a wallet from an existing keypair.

**Parameters**:
- `public_key` (string) - Public key
- `private_key` (string) - Private key

**Returns**: 
```json
{
  "success": true,
  "address": "synw...",
  "message": "Wallet created successfully"
}
```
or error object

---

### `synergy_getAllWallets`
Get all wallets.

**Parameters**: None

**Returns**: Array of wallet objects

---

### `synergy_signTransaction`
Sign a transaction with a wallet.

**Parameters**:
- `address` (string) - Wallet address
- `transaction` (object) - Transaction object to sign

**Returns**: 
```json
{
  "success": true,
  "message": "Transaction signed successfully",
  "transaction": {...}
}
```
or error object

---

## Network Methods

### `synergy_getNetworkStats`
Get network statistics.

**Parameters**: None

**Returns**: Stats object with:
- `block_height`: Current block height
- `total_transactions`: Total number of transactions
- `active_validators`: Number of active validators
- `total_supply`: Total token supply
- `tokens`: Number of tokens
- `network_uptime`: Network uptime percentage
- `current_epoch`: Current epoch number
- `total_staked`: Total staked amount

---

### `synergy_getPeerInfo`
Get peer information.

**Parameters**: None

**Returns**: Object with:
- `peer_count`: Number of connected peers
- `peers`: Array of peer information objects with:
  - `address`: Peer address
  - `node_id`: Node ID
  - `version`: Peer version
  - `capabilities`: Peer capabilities
  - `last_seen`: Last seen timestamp
  - `blocks_sent`: Blocks sent
  - `blocks_received`: Blocks received
  - `txs_sent`: Transactions sent
  - `txs_received`: Transactions received

---

## SXCP (Synergy Cross-Chain Protocol) Methods

### `synergy_registerRelayer`
Register a new SXCP relayer.

**Parameters**:
- `address` (string) - Relayer address
- `public_key` (string) - Relayer public key (base64 encoded)

**Returns**: 
```json
{
  "success": true,
  "message": "Relayer registered",
  "quorum": { "n": 3, "t": 2 }
}
```

---

### `synergy_unregisterRelayer`
Deactivate a relayer.

**Parameters**:
- `address` (string) - Relayer address

**Returns**: 
```json
{
  "success": true,
  "message": "Relayer deactivated",
  "quorum": { "n": 2, "t": 2 }
}
```

---

### `synergy_relayerHeartbeat`
Submit a relayer heartbeat.

**Parameters**:
- `address` (string) - Relayer address

**Returns**: 
```json
{
  "success": true,
  "message": "Heartbeat recorded",
  "quorum": { "n": 3, "t": 2 }
}
```

---

### `synergy_getRelayerSet`
Get the current set of registered relayers.

**Parameters**: None

**Returns**: 
```json
{
  "relayers": [...],
  "heartbeat_timeout_secs": 120,
  "quorum": { "n": 3, "t": 2 }
}
```

---

### `synergy_getRelayerHealth`
Get health status of all relayers.

**Parameters**: None

**Returns**: 
```json
{
  "relayers": [
    {
      "address": "...",
      "active": true,
      "slashed": false,
      "online": true,
      "heartbeat_age_secs": 30,
      "heartbeat_timeout_secs": 120,
      "eligible_for_quorum": true,
      "reputation": 10,
      "attestation_count": 5
    }
  ],
  "quorum": { "n": 3, "t": 2 }
}
```

---

### `synergy_getSxcpStatus`
Get overall SXCP status.

**Parameters**: None

**Returns**: 
```json
{
  "quorum": { "n": 3, "t": 2 },
  "heartbeat_timeout_secs": 120,
  "relayer_totals": {
    "registered": 5,
    "active": 4,
    "online": 3,
    "slashed": 1
  },
  "event_totals": {
    "tracked": 10,
    "pending": 2,
    "finalized": 8
  },
  "attestation_count": 8,
  "slashing_event_count": 1
}
```

---

### `synergy_submitAttestation`
Submit an attestation for a cross-chain event.

**Parameters**:
- `submitted_by` (string) - Relayer address submitting the attestation
- `event_hash` (string) - Hash of the event being attested
- `aggregate_sig` (string) - Aggregate signature (base64 encoded)
- `metadata` (object, optional) - Additional metadata

**Returns**: 
```json
{
  "success": true,
  "message": "Attestation support recorded",
  "finalized": false,
  "event_hash": "...",
  "support_count": 1,
  "threshold": 2,
  "quorum": { "n": 3, "t": 2 },
  "timestamp": 1640995200
}
```

---

### `synergy_getEventAttestation`
Get attestation status for a specific event.

**Parameters**:
- `event_hash` (string) - Event hash

**Returns**: 
```json
{
  "success": true,
  "event": {...},
  "all_supporters": [...],
  "eligible_supporters": [...],
  "support_count": 2,
  "threshold": 2,
  "quorum": { "n": 3, "t": 2 }
}
```

---

### `synergy_getAttestations`
Get recent attestations.

**Parameters**:
- `limit` (number, optional) - Maximum number of attestations to return (default: 100)

**Returns**: 
```json
{
  "attestations": [...],
  "count": 100
}
```

---

### `synergy_slashRelayer`
Slash a relayer for misbehavior.

**Parameters**:
- `address` (string) - Relayer address
- `reason` (string) - Reason for slashing
- `penalty` (number, optional) - Penalty amount (default: 25)

**Returns**: 
```json
{
  "success": true,
  "message": "Relayer slashed",
  "relayer": "...",
  "reason": "...",
  "penalty": 25,
  "newly_finalized_events": [...],
  "quorum": { "n": 2, "t": 2 }
}
```

---

### `synergy_setSxcpHeartbeatTimeout`
Set the SXCP heartbeat timeout.

**Parameters**:
- `timeout_secs` (number) - Timeout in seconds (minimum: 10)

**Returns**: 
```json
{
  "success": true,
  "heartbeat_timeout_secs": 120,
  "quorum": { "n": 3, "t": 2 }
}
```

---

### `synergy_resetSxcpState`
Reset SXCP state (requires confirmation token).

**Parameters**:
- `confirmation_token` (string) - Must be "TESTBETA_RESET_SXCP_STATE"

**Returns**: 
```json
{
  "success": true,
  "message": "SXCP state reset"
}
```

---

## Core Blockchain Methods (Phase 1)

### `synergy_getTransactionReceipt`
Get a transaction receipt with execution details.

**Parameters**:
- `txHash` (string) - Transaction hash

**Returns**: Transaction receipt object with:
- `transactionHash`: Transaction hash
- `transactionIndex`: Index in block
- `blockHash`: Block hash
- `blockNumber`: Block number
- `from`: Sender address
- `to`: Recipient address (or null for contract creation)
- `cumulativeGasUsed`: Total gas used in block up to this tx
- `gasUsed`: Gas used by this transaction
- `effectiveGasPrice`: Actual gas price
- `status`: `"0x1"` for success
- `logs`: Event logs array
- `logsBloom`: Bloom filter for logs
- `contractAddress`: Created contract address (if applicable, null otherwise)

**Note**: Returns `null` for pending (unmined) transactions.

---

### `synergy_getTransactionCount`
Get the transaction count (nonce) for an address.

**Parameters**:
- `address` (string) - Address to query
- `blockTag` (string, optional) - `"latest"` or `"pending"` (default: `"latest"`)

**Returns**: `number` - Transaction count/nonce

**Note**: When `blockTag` is `"pending"`, includes transactions in the mempool.

---

### `synergy_getBalance`
Get the SNRG balance for an address (standardized method).

**Parameters**:
- `address` (string) - Address to query

**Returns**: `number` - Balance in nWei

**Note**: This is a standardized balance query for the native SNRG token. For other tokens, use `synergy_getTokenBalance`.

---

### `synergy_gasPrice`
Get the current gas price based on recent network utilization.

**Parameters**: None

**Returns**: `number` - Current gas price in nWei per gas unit

**Note**: The gas price is dynamically calculated based on the utilization of the last 10 blocks. Default is 40 nWei, clamped between 1 and 1000 nWei.

---

### `synergy_call`
Execute a contract call locally (read-only, no state change).

**Parameters**:
- `callObject` (object) - Call object with:
  - `from`: Sender address (optional)
  - `to`: Contract address (required)
  - `data`: Calldata hex (optional)
  - `value`: Value to send (optional)

**Returns**: Object with:
- `result`: Return value (hex, `"0x"` when AIVM is disabled)
- `note`: Status message

**Note**: AIVM contract execution is currently disabled in testnet-beta. Contract calls return empty results until AIVM is re-enabled.

---

### `synergy_estimateGas`
Estimate the gas required for a transaction.

**Parameters**:
- `transaction` (object) - Transaction object with:
  - `from`: Sender address (optional)
  - `to`: Recipient address (null for contract deploy)
  - `data`: Transaction data hex (optional)
  - `value`: Value to send (optional)

**Returns**: `number` - Estimated gas amount

**Gas Estimates**:
- Simple transfer: 21,000 gas
- Contract deployment: 500,000 + 200 gas/byte of bytecode
- Contract call: 100,000 + 68 gas/byte of calldata

---

### `synergy_getLogs`
Get event logs matching filters.

**Parameters**:
- `filter` (object) - Filter object with:
  - `fromBlock` (number, optional) - Starting block (default: 0)
  - `toBlock` (number, optional) - Ending block (default: latest)
  - `address` (string, optional) - Filter by address
  - `topics` (array, optional) - Event topics to filter
  - `blockHash` (string, optional) - Specific block hash

**Returns**: Array of log objects with:
- `logIndex`: Log index
- `transactionIndex`: Transaction index in block
- `transactionHash`: Transaction hash
- `blockHash`: Block hash
- `blockNumber`: Block number
- `address`: Emitting address
- `data`: Log data
- `topics`: Log topics
- `removed`: Whether log was removed (always `false`)

**Note**: Full EVM-style event logs will be available when AIVM is re-enabled.

---

### `synergy_getCode`
Get the code (smart contract bytecode) stored at an address.

**Parameters**:
- `address` (string) - Contract address

**Returns**: `string` - Contract bytecode (hex) or `"0x"` if not a contract

**Note**: Returns `"0x"` for all addresses while AIVM is disabled.

---

### `synergy_getStorageAt`
Get the value from a storage position of a contract/account.

**Parameters**:
- `address` (string) - Contract/account address
- `position` (string) - Storage slot position (hex)
- `blockTag` (string, optional) - `"latest"`, `"pending"`, or block number

**Returns**: `string` - Storage value (32 bytes, hex-encoded with `0x` prefix)

**Note**: Returns zero bytes for all positions while AIVM is disabled.

---

### `synergy_getBlockTransactionCount`
Get the number of transactions in a block.

**Parameters**:
- `blockNumber` (number) OR `blockHash` (string) - Block identifier

**Returns**: `number` - Transaction count, or `null` if block not found

---

### `synergy_getBlockReceipts`
Get all transaction receipts for a block.

**Parameters**:
- `blockNumber` (number) OR `blockHash` (string) - Block identifier

**Returns**: Array of transaction receipt objects (same format as `synergy_getTransactionReceipt`), or `null` if block not found

---

### `synergy_getPendingTransactions`
Get detailed information about pending transactions with sorting.

**Parameters**:
- `limit` (number, optional) - Maximum transactions to return (default: 100)
- `sortBy` (string, optional) - Sort field: `"gasPrice"`, `"nonce"`, or `"timestamp"` (default: `"timestamp"`)

**Returns**: Array of pending transaction objects sorted by the specified field

---

### `synergy_getTransactionByBlockNumberAndIndex`
Get a transaction by block number and index.

**Parameters**:
- `blockNumber` (number) - Block number
- `index` (number) - Transaction index within the block

**Returns**: Transaction object or `null` if not found

---

### `synergy_getTransactionByBlockHashAndIndex`
Get a transaction by block hash and index.

**Parameters**:
- `blockHash` (string) - Block hash
- `index` (number) - Transaction index within the block

**Returns**: Transaction object or `null` if not found

---

### `synergy_maxFeePerGas`
Get the maximum fee per gas.

**Parameters**: None

**Returns**: `number` - Max fee per gas (2x default gas price)

---

### `synergy_maxPriorityFeePerGas`
Get the maximum priority fee per gas.

**Parameters**: None

**Returns**: `number` - Max priority fee per gas (1/4 of default gas price)

---

### `synergy_getFeeHistory`
Get historical gas fee information.

**Parameters**:
- `blockCount` (number, optional) - Number of blocks to include (default: 10)
- `newestBlock` (string, optional) - Highest block tag (default: `"latest"`)
- `rewardPercentiles` (array, optional) - Percentiles to calculate (e.g., `[25, 50, 75]`)

**Returns**: Fee history object with:
- `baseFeePerGas`: Array of base fees per block
- `gasUsedRatio`: Array of gas utilization ratios per block
- `reward`: Array of reward arrays at requested percentiles
- `oldestBlock`: Oldest block number in the result

---

## Enhanced Validator & Staking Methods (Phase 2)

### `synergy_getChainId`
Get the chain ID.

**Parameters**: None

**Returns**: `number` - Chain ID (338639 for testnet-beta)

---

### `synergy_getValidatorByCluster`
Get all validators in a specific cluster.

**Parameters**:
- `clusterId` (number) - Cluster ID

**Returns**: Array of validator objects filtered by cluster membership

---

### `synergy_getValidatorRewards`
Get rewards earned by a validator over time.

**Parameters**:
- `address` (string) - Validator address
- `fromEpoch` (number, optional) - Starting epoch
- `toEpoch` (number, optional) - Ending epoch

**Returns**: Object with:
- `address`: Validator address
- `totalBlocksProduced`: Total blocks produced
- `rewards`: Array of reward objects (blockNumber, amount, type, timestamp)
- `totalRewards`: Sum of all rewards

---

### `synergy_getValidatorPerformance`
Get detailed performance metrics for a validator.

**Parameters**:
- `address` (string) - Validator address

**Returns**: Performance object with:
- `attestationSuccessRate`: Uptime percentage
- `blockProposalSuccessRate`: Block proposal success rate
- `averageInclusionDelay`: Average block time
- `missedAttestations`: Missed blocks count
- `orphanedBlocks`: Orphaned blocks count
- `effectiveBalance`: Current stake amount
- `totalBlocksProduced`: Blocks produced count
- `synergyScore`: Current synergy score
- `reputationScore`: Reputation score
- `collaborationScore`: Collaboration score
- `uptime`: Uptime percentage

---

### `synergy_getValidatorQueue`
Get the validator activation/deactivation queue status.

**Parameters**: None

**Returns**: Queue object with:
- `activationQueue`: Array of pending validator registrations
- `activationQueueLength`: Number of validators waiting
- `exitQueue`: Array of jailed validators
- `exitQueueLength`: Number of validators exiting
- `estimatedActivationTime`: Estimated activation timestamp
- `estimatedExitTime`: Estimated exit timestamp

---

### `synergy_requestValidatorExit`
Request a validator to exit (initiate unstaking period).

**Parameters**:
- `address` (string) - Validator address
- `signature` (string) - Signed exit request

**Returns**:
```json
{
  "success": true,
  "message": "Validator exit requested",
  "validatorAddress": "synv...",
  "currentEpoch": 10,
  "exitEpoch": 12,
  "withdrawalAvailableAt": 1640995200
}
```

---

### `synergy_getValidatorSlashingHistory`
Get slashing history for a validator.

**Parameters**:
- `address` (string) - Validator address

**Returns**: Object with:
- `address`: Validator address
- `slashingEvents`: Array of slashing event objects
- `totalPenalties`: Total penalty amount
- `doubleSignCount`: Number of double-sign infractions

---

### `synergy_getClusterInfo`
Get detailed information about a validator cluster.

**Parameters**:
- `clusterId` (number) - Cluster ID

**Returns**: Cluster info object with:
- `clusterId`: Cluster ID
- `address`: Cluster address
- `validators`: Array of validator detail objects
- `validatorCount`: Number of validators
- `totalStake`: Total stake in cluster
- `averageSynergyScore`: Average synergy score
- `createdAt`: Creation timestamp
- `lastRotation`: Last rotation timestamp
- `group`: Cluster group number

---

### `synergy_getClusterRewards`
Get rewards distribution for a cluster.

**Parameters**:
- `clusterId` (number) - Cluster ID
- `epoch` (number, optional) - Specific epoch (default: current)

**Returns**: Object with:
- `clusterId`: Cluster ID
- `epoch`: Epoch number
- `totalRewards`: Total rewards for cluster
- `distributions`: Array of per-validator reward objects

---

### `synergy_proposeClusterChange`
Propose a change to cluster composition.

**Parameters**:
- `clusterId` (number) - Cluster ID
- `proposal` (object) - Proposal details
- `proposer` (string) - Proposer address (must be a registered validator)

**Returns**:
```json
{
  "success": true,
  "proposalId": "prop_1_1640995200",
  "clusterId": 1,
  "proposer": "synv...",
  "votingEndsAt": 1641081600
}
```

---

### `synergy_getStakingRewards`
Get staking rewards for an address.

**Parameters**:
- `address` (string) - Staker address
- `validator` (string, optional) - Filter by specific validator

**Returns**: Object with:
- `address`: Staker address
- `rewards`: Array of reward objects (validator, stakedAmount, rewardsEarned, stakingStart, isActive)
- `totalRewardsEarned`: Sum of all rewards earned

---

### `synergy_getStakingAPY`
Get current staking APY information.

**Parameters**:
- `validator` (string, optional) - Specific validator for validator-specific APY

**Returns**: APY object with:
- `currentAPY`: Current annual percentage yield (capped at 20%)
- `averageAPY`: Average APY
- `networkStakingRate`: Percentage of total supply staked
- `totalStaked`: Total staked amount
- `totalSupply`: Total token supply
- `baseAPY`: Base annual yield (5%)
- `validatorAPY`: Validator-specific APY (if validator specified)
- `validatorSynergyScore`: Validator's synergy score (if validator specified)

**Note**: APY is inversely proportional to staking participation rate, incentivizing staking when participation is low.

---

### `synergy_getDelegatedStakes`
Get all delegated stakes for a staker.

**Parameters**:
- `address` (string) - Staker address

**Returns**: Object with:
- `address`: Staker address
- `delegations`: Array of active delegation objects (validator, amount, rewardsEarned, delegatedAt)
- `totalDelegated`: Total amount delegated

---

### `synergy_getDelegators`
Get all delegators for a validator.

**Parameters**:
- `validator` (string) - Validator address
- `limit` (number, optional) - Maximum results (default: 100)

**Returns**: Object with:
- `validator`: Validator address
- `delegators`: Array of delegator objects sorted by amount (descending)
- `totalDelegators`: Number of delegators

---

### `synergy_claimRewards`
Claim accumulated staking rewards.

**Parameters**:
- `staker` (string) - Staker address
- `validator` (string, optional) - Specific validator (claims all if not specified)

**Returns**:
```json
{
  "success": true,
  "claimedAmount": 1000000,
  "stakerAddress": "synw...",
  "message": "Rewards claimed successfully"
}
```

---

### `synergy_getRewardsProjection`
Project future staking rewards.

**Parameters**:
- `address` (string) - Staker address
- `amount` (number) - Stake amount to project
- `duration` (number) - Duration in days
- `validator` (string, optional) - Specific validator

**Returns**: Projection object with:
- `stakeAmount`: Input stake amount
- `durationDays`: Input duration
- `estimatedAPY`: Calculated APY
- `projectedReward`: Estimated reward over period
- `projectedTotal`: Stake + projected reward

---

### `synergy_getUnstakingPeriod`
Get information about the unstaking period.

**Parameters**: None

**Returns**: Object with:
- `unstakingPeriodDays`: 7 days
- `unstakingPeriodSeconds`: 604800 seconds
- `currentQueueLength`: Current unstaking queue length
- `estimatedWithdrawalTime`: Estimated withdrawal timestamp

---

## Planned Methods

### `synergy_getUncleByBlockNumberAndIndex` *(Planned)*
Get uncle block information (for future PoW hybrid support).

**Parameters**:
- `blockNumber` (number)
- `uncleIndex` (number)

**Returns**: Uncle block object or `null`

**Note**: Reserved for future PoW hybrid support. Currently returns `null`.

---

### `synergy_getUncleCountByBlockNumber` *(Planned)*
Get the number of uncles in a block.

**Parameters**:
- `blockNumber` (number)

**Returns**: `number` - Uncle count (currently always `0`)

---

### `synergy_getProof` *(Planned)*
Get Merkle proof for an account or storage slot.

**Parameters**:
- `address` (string) - Address
- `storageKeys` (array) - Storage keys to prove
- `blockTag` (string, optional) - Block tag

**Returns**: Account proof object with storage proofs

**Use Case**: Light client verification, state proofs.

---

### `synergy_getFilterLogs` *(Planned)*
Get logs for a specific filter (for poll-based subscriptions).

**Parameters**:
- `filterId` (string) - Filter ID

**Returns**: Array of log objects

**Note**: Requires filter registration support. Will be implemented alongside WebSocket subscriptions.

---

### `synergy_getFilterChanges` *(Planned)*
Get new logs since last poll for a filter.

**Parameters**:
- `filterId` (string) - Filter ID

**Returns**: Array of new log objects since last poll

**Note**: Requires filter registration support. Will be implemented alongside WebSocket subscriptions.

---

## Error Responses

If an error occurs, the response will be:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Error message"
  }
}
```

Common error codes:
- `-32600`: Invalid Request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32000`: Server error

---

## Notes

1. **SNRG Denomination**: All amounts are stored internally as nWei (1 SNRG = 1,000,000,000 nWei). The `synergy_sendTokens` and staking methods accept amounts in SNRG and convert them automatically.

2. **Transaction Hashes**: Transaction hashes can be provided in multiple formats:
   - Full format: `syntxn-a0d53ef9...`
   - Raw format: `a0d53ef9...`
   - With or without `0x` prefix

3. **Timestamps**: All timestamps are Unix timestamps in seconds.

4. **AIVM Methods**: AIVM (Artificial Intelligence Virtual Machine) methods are currently disabled in the testnet-beta.

5. **Network IDs**: 
   - Testnet-Beta Network ID: 338639
   - Testnet-Beta Chain ID: 338639

6. **Address Formats**:
   - Wallet addresses: `synw...` (bech32m encoded)
   - Validator addresses: `synv...` (bech32m encoded)

---

## Example Usage

### Get Current Block Number
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_getBlockNumber","params":[],"id":1}'
```

### Get Block by Number
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_getBlockByNumber","params":[150],"id":1}'
```

### Send Tokens
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"synergy_sendTokens",
    "params":[
      "synw1lfgerdqglc6p74p9u6k8ghfssl59q8jzhuwm07",
      "synv11gfutu5quzf9jc7tra0x4c95shaagaywxc6zae3",
      "SNRG",
      500000
    ],
    "id":1
  }'
```

### Get Transaction by Hash
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"synergy_getTransactionByHash",
    "params":["syntxn-a0d53ef9548a458f16cfc6ae0f5e27a40765419633f19b9bf4e44a4cf4ace82d"],
    "id":1
  }'
```

### Get Node Status
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_getNodeStatus","params":[],"id":1}'
```

### Register SXCP Relayer
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"synergy_registerRelayer",
    "params":["synr...", "base64-encoded-public-key"],
    "id":1
  }'
```

### Get Validator Stats
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_getValidatorStats","params":[],"id":1}'
```

### Get Network Stats
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_getNetworkStats","params":[],"id":1}'
```
