# Synergy Network API Reference

## Overview

The Synergy Network provides a comprehensive JSON-RPC API for interacting with the blockchain. All API methods are accessible via HTTP POST requests to the RPC server running on port 8545.

## Base URL

```
http://localhost:8545
```

## Request Format

All requests should be HTTP POST with JSON content:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_methodName",
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
    "code": -32600,
    "message": "Invalid Request"
  },
  "id": 1
}
```

## API Methods

### Blockchain Queries

#### `synergy_blockNumber`
Returns the latest block number.

**Parameters:** None

**Returns:**
```json
{
  "block_number": 123
}
```

#### `synergy_getBlockByNumber`
Returns block information by block number.

**Parameters:**
- `block_number` (integer): The block number to query

**Returns:**
```json
{
  "block_index": 123,
  "timestamp": 1640995200,
  "transactions": [...],
  "validator": "sYn...",
  "hash": "..."
}
```

#### `synergy_getLatestBlock`
Returns the latest block information.

**Parameters:** None

**Returns:** Same as `synergy_getBlockByNumber`

### Transaction Methods

#### `synergy_sendTransaction`
Submits a transaction to the network.

**Parameters:**
- `transaction` (object): Transaction object with fields:
  - `sender`: Sender address
  - `receiver`: Receiver address
  - `amount`: Transaction amount
  - `nonce`: Transaction nonce
  - `gas_price`: Gas price
  - `gas_limit`: Gas limit
  - `data`: Optional transaction data

**Returns:**
```json
{
  "success": true,
  "message": "Transaction submitted successfully"
}
```

#### `synergy_getTransactionPool`
Returns all pending transactions in the pool.

**Parameters:** None

**Returns:**
```json
[
  {
    "sender": "sYn...",
    "receiver": "sYn...",
    "amount": 1000,
    "nonce": 1,
    "gas_price": 1000,
    "gas_limit": 21000,
    "signature": "...",
    "data": "...",
    "timestamp": 1640995200
  }
]
```

#### `synergy_getTransactionByHash`
Returns transaction information by hash.

**Parameters:**
- `transaction_hash` (string): Transaction hash

**Returns:** Transaction object or null if not found

### Token Operations

#### `synergy_createToken`
Creates a new token on the network.

**Parameters:**
- `symbol` (string): Token symbol
- `name` (string): Token name
- `decimals` (integer): Number of decimal places
- `total_supply` (integer): Initial token supply
- `creator` (string): Creator address

**Returns:**
```json
{
  "success": true,
  "message": "Token TOKEN created"
}
```

#### `synergy_mintTokens`
Mints new tokens to an address.

**Parameters:**
- `to` (string): Recipient address
- `token_symbol` (string): Token symbol
- `amount` (integer): Amount to mint

**Returns:**
```json
{
  "success": true,
  "message": "Minted 1000 TOKEN to sYn..."
}
```

#### `synergy_burnTokens`
Burns tokens from an address.

**Parameters:**
- `from` (string): Address to burn from
- `token_symbol` (string): Token symbol
- `amount` (integer): Amount to burn

**Returns:**
```json
{
  "success": true,
  "message": "Burned 1000 TOKEN from sYn..."
}
```

#### `synergy_transferTokens`
Transfers tokens between addresses.

**Parameters:**
- `from` (string): Sender address
- `to` (string): Recipient address
- `token_symbol` (string): Token symbol
- `amount` (integer): Amount to transfer

**Returns:**
```json
{
  "success": true,
  "message": "Transferred 1000 TOKEN from sYn... to sYn..."
}
```

#### `synergy_getTokenBalance`
Returns token balance for an address.

**Parameters:**
- `address` (string): Address to query
- `token_symbol` (string): Token symbol

**Returns:**
```json
{
  "balance": 1000
}
```

#### `synergy_getAllBalances`
Returns all token balances for an address.

**Parameters:**
- `address` (string): Address to query

**Returns:**
```json
{
  "SNRG": 1000,
  "TOKEN": 500
}
```

#### `synergy_getTokens`
Returns all available tokens.

**Parameters:** None

**Returns:**
```json
[
  {
    "symbol": "SNRG",
    "name": "SynergyCoin",
    "decimals": 18,
    "total_supply": 1000000000000000000000,
    "max_supply": 2000000000000000000000,
    "mintable": true,
    "burnable": true,
    "created_at": 1640995200,
    "creator": "genesis"
  }
]
```

#### `synergy_getTransferHistory`
Returns transfer history for an address.

**Parameters:**
- `address` (string): Address to query
- `limit` (integer, optional): Maximum number of results (default: 50)

**Returns:**
```json
[
  {
    "from": "sYn...",
    "to": "sYn...",
    "token_symbol": "SNRG",
    "amount": 1000,
    "fee": 1000,
    "timestamp": 1640995200,
    "tx_hash": "...",
    "block_height": 123
  }
]
```

### Staking Operations

#### `synergy_stakeTokens`
Creates a staking transaction through wallet manager.

**Parameters:**
- `staker` (string): Staker address
- `validator` (string): Validator address
- `token_symbol` (string): Token symbol
- `amount` (integer): Amount to stake

**Returns:**
```json
{
  "success": true,
  "transaction": {...},
  "message": "Staking transaction created successfully"
}
```

#### `synergy_stakeTokensDirect`
Stakes tokens directly.

**Parameters:**
- `staker` (string): Staker address
- `validator` (string): Validator address
- `token_symbol` (string): Token symbol
- `amount` (integer): Amount to stake

**Returns:**
```json
{
  "success": true,
  "message": "Staked 1000 SNRG to validator sYn..."
}
```

#### `synergy_unstakeTokens`
Unstakes tokens from a validator.

**Parameters:**
- `staker` (string): Staker address
- `validator` (string): Validator address
- `token_symbol` (string): Token symbol
- `amount` (integer): Amount to unstake

**Returns:**
```json
{
  "success": true,
  "message": "Unstaked 1000 SNRG from validator sYn..."
}
```

#### `synergy_getStakedBalance`
Returns staked balance for an address.

**Parameters:**
- `address` (string): Address to query
- `token_symbol` (string): Token symbol

**Returns:**
```json
{
  "balance": 1000
}
```

#### `synergy_getStakingInfo`
Returns staking information for an address.

**Parameters:**
- `address` (string): Address to query

**Returns:**
```json
[
  {
    "validator_address": "sYn...",
    "staker_address": "sYn...",
    "amount": 1000,
    "stake_start": 1640995200,
    "stake_end": null,
    "rewards_earned": 50,
    "is_active": true
  }
]
```

### Wallet Management

#### `synergy_createWallet`
Creates a new wallet.

**Parameters:** None

**Returns:**
```json
{
  "address": "sYn...",
  "message": "Wallet created successfully"
}
```

#### `synergy_createWalletFromKeypair`
Creates a wallet from existing keypair.

**Parameters:**
- `public_key` (string): Public key
- `private_key` (string): Private key

**Returns:**
```json
{
  "success": true,
  "address": "sYn...",
  "message": "Wallet created successfully"
}
```

#### `synergy_getWallet`
Returns wallet information.

**Parameters:**
- `address` (string): Wallet address

**Returns:**
```json
{
  "address": "sYn...",
  "public_key": "...",
  "balance": {...},
  "staked_balance": {...},
  "nonce": 1,
  "created_at": 1640995200
}
```

#### `synergy_getAllWallets`
Returns all wallets.

**Parameters:** None

**Returns:** Array of wallet objects

#### `synergy_signTransaction`
Signs a transaction.

**Parameters:**
- `address` (string): Wallet address
- `transaction` (object): Transaction to sign

**Returns:**
```json
{
  "success": true,
  "message": "Transaction signed successfully",
  "transaction": {...}
}
```

### Validator Management

#### `synergy_registerValidator`
Registers a new validator.

**Parameters:**
- `address` (string): Validator address
- `public_key` (string): Validator public key
- `name` (string): Validator name
- `stake_amount` (integer): Stake amount

**Returns:**
```json
{
  "success": true,
  "message": "Validator registration submitted successfully"
}
```

#### `synergy_approveValidator`
Approves a pending validator registration.

**Parameters:**
- `address` (string): Validator address

**Returns:**
```json
{
  "success": true,
  "message": "Validator approved successfully"
}
```

#### `synergy_getValidators`
Returns all validators.

**Parameters:** None

**Returns:** Array of validator objects

#### `synergy_getValidator`
Returns validator information.

**Parameters:**
- `address` (string): Validator address

**Returns:** Validator object or null

#### `synergy_getTopValidators`
Returns top validators by synergy score.

**Parameters:**
- `count` (integer, optional): Number of validators (default: 10)

**Returns:** Array of validator objects

#### `synergy_getValidatorActivity`
Returns validator activity statistics.

**Parameters:** None

**Returns:**
```json
{
  "validators": [
    {
      "address": "synv1...",
      "name": "Validator Name",
      "synergy_score": 95.2,
      "blocks_produced": 120,
      "uptime": "99.8%",
      "cluster_id": 0,
      "stake_amount": 1000000000000,
      "last_active": 1640995200
    }
  ],
  "total_active": 1,
  "average_synergy_score": 95.2
}
```

#### `synergy_getSynergyScoreBreakdown`
Returns detailed Synergy Score components for a validator.

**Parameters:**
- `address` (string): Validator address

**Returns:**
```json
{
  "address": "synv1...",
  "total_score": 92.5,
  "components": {
    "stake_weight": 0.05,
    "reputation": 0.98,
    "contribution_index": 12.4,
    "cartelization_penalty": 0.0,
    "normalized_score": 92.5,
    "last_updated": 1640995200
  }
}
```

#### `synergy_slashValidator`
Slashes a validator.

**Parameters:**
- `address` (string): Validator address
- `reason` (string): Slashing reason

**Returns:**
```json
{
  "success": true,
  "message": "Validator slashed successfully"
}
```

### Network Information

#### `synergy_nodeInfo`
Returns node information.

**Parameters:** None

**Returns:**
```json
{
  "name": "Synergy Testnet Node",
  "version": "1.0.0",
  "protocolVersion": 1,
  "networkId": 1264,
  "chainId": 1264,
  "consensus": "Proof of Synergy",
  "syncing": false,
  "currentBlock": 123,
  "timestamp": 1640995200
}
```

#### `synergy_getNetworkStats`
Returns comprehensive network statistics.

**Parameters:** None

**Returns:**
```json
{
  "block_height": 123,
  "total_transactions": 456,
  "active_validators": 10,
  "total_supply": 1000000000000000000000,
  "tokens": 2,
  "network_uptime": "99.9%",
  "current_epoch": 1,
  "total_staked": 500000000000000000000
}
```

#### `synergy_getValidatorStats`
Returns comprehensive validator statistics.

**Parameters:** None

**Returns:**
```json
{
  "total_validators": 10,
  "active_validators": [...],
  "top_validators": [...],
  "epoch_rewards": {...}
}
```

#### `synergy_getTokenStats`
Returns comprehensive token statistics.

**Parameters:** None

**Returns:**
```json
[
  {
    "symbol": "SNRG",
    "name": "SynergyCoin",
    "total_supply": 1000000000000000000000,
    "total_staked": 500000000000000000000,
    "holders": 5
  }
]
```

### Explorer Data

#### `synergy_getBlockRange`
Returns blocks in a range.

**Parameters:**
- `start` (integer): Start block number
- `end` (integer): End block number

**Returns:** Array of block objects

#### `synergy_getTransactionsInBlock`
Returns all transactions in a block.

**Parameters:**
- `block_number` (integer): Block number

**Returns:** Array of transaction objects

## Error Codes

- `-32700`: Parse error
- `-32600`: Invalid Request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

## Examples

### Create a Token
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "synergy_createToken",
    "params": ["MYTOKEN", "My Token", 18, 1000000, "sYn..."],
    "id": 1
  }'
```

### Transfer Tokens
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "synergy_transferTokens",
    "params": ["sYn...", "sYn...", "SNRG", 1000],
    "id": 1
  }'
```

### Get Network Stats
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "synergy_getNetworkStats",
    "params": [],
    "id": 1
  }'
```
