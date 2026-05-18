# Synergy Testnet RPC Methods To Implement

This document lists all RPC methods that should be considered for implementation in the Synergy Testnet network. These methods are organized by priority and category.

> **Phase 1 (Priority 1: Core Blockchain Functionality) has been completed and moved to `rpc-methods.md`.**

---

> **Phase 2 (Priority 2: Enhanced Validator & Staking) has been completed and moved to `rpc-methods.md`.**

---

## Priority 3: Network & P2P

### Network Information

#### `synergy_getChainId`
Get the chain ID.

**Parameters**: None

**Returns**: `number` - Chain ID (1262 for testnet)

---

#### `synergy_getNetworkVersion`
Get the network version/protocol version.

**Parameters**: None

**Returns**: `string` - Network version

---

#### `synergy_getBootnodes`
Get the list of bootnodes.

**Parameters**: None

**Returns**: Array of bootnode objects with:
- `address`: Bootnode address
- `port`: Port number
- `publicKey`: Public key
- `status`: Online/offline status

---

#### `synergy_addPeer`
Manually add a peer to connect to.

**Parameters**:
- `enode` (string) - Enode URL of the peer

**Returns**: 
```json
{
  "success": true,
  "message": "Peer added"
}
```

---

#### `synergy_removePeer`
Remove a peer from the connection list.

**Parameters**:
- `peerId` (string) - Peer ID

**Returns**: 
```json
{
  "success": true,
  "message": "Peer removed"
}
```

---

#### `synergy_getPeerDetails`
Get detailed information about a specific peer.

**Parameters**:
- `peerId` (string) - Peer ID

**Returns**: Detailed peer information including connection stats, capabilities, and history

---

#### `synergy_getNetworkDifficulty`
Get the current network difficulty (for PoW hybrid or future use).

**Parameters**: None

**Returns**: `number` - Current difficulty

---

#### `synergy_getHashrate`
Get the current network hashrate (for PoW hybrid or future use).

**Parameters**: None

**Returns**: `number` - Current hashrate

---

#### `synergy_getNodeDiscoveryTable`
Get the current Kademlia routing table.

**Parameters**: None

**Returns**: Discovery table with peer buckets organized by distance

---

#### `synergy_ping`
Ping the node to check connectivity.

**Parameters**: None

**Returns**: 
```json
{
  "status": "ok",
  "latency_ms": 5
}
```

---

#### `synergy_getNodeUptime`
Get detailed node uptime statistics.

**Parameters**: None

**Returns**: Object with:
- `totalUptimeSeconds`: Total uptime in seconds
- `uptimePercentage`: Uptime percentage
- `lastRestart`: Last restart timestamp
- `restartCount`: Number of restarts
- `averageSessionDuration`: Average session duration

---

### Subscription & WebSocket Methods

#### `synergy_subscribe`
Subscribe to events (WebSocket only).

**Parameters**:
- `subscriptionType` (string) - Type of subscription:
  - `newHeads`: New block headers
  - `logs`: Event logs
  - `pendingTransactions`: Pending transactions
  - `validatorEvents`: Validator events
- `filter` (object, optional) - Filter parameters

**Returns**: `string` - Subscription ID

---

#### `synergy_unsubscribe`
Unsubscribe from events.

**Parameters**:
- `subscriptionId` (string) - Subscription ID

**Returns**: `boolean` - Success status

---

#### `synergy_subscription`
Notification method for subscription events (server-to-client).

**Parameters**: None (push notification)

**Returns**: Event data based on subscription type

---

## Priority 4: Smart Contracts & AIVM

### Contract Methods (AIVM - When Re-enabled)

#### `synergy_deployAIVMContract`
Deploy an AIVM smart contract.

**Parameters**:
- `bytecode` (string) - Contract bytecode (hex)
- `abi` (string) - Contract ABI (JSON string)
- `contractType` (string) - Type: "ai", "cross_chain", "oracle", "standard"
- `deployer` (string) - Deployer address
- `constructorArgs` (array, optional) - Constructor arguments

**Returns**: 
```json
{
  "success": true,
  "contractAddress": "sync...",
  "transactionHash": "syntxn-...",
  "message": "Contract deployed successfully"
}
```

---

#### `synergy_executeAIVMContract`
Execute an AIVM contract function.

**Parameters**:
- `contractAddress` (string) - Contract address
- `functionName` (string) - Function to call
- `args` (array) - Function arguments
- `sender` (string) - Sender address
- `value` (number, optional) - Value to send
- `gasLimit` (number, optional) - Gas limit

**Returns**: Execution result with return values and gas used

---

#### `synergy_getAIVMContract`
Get AIVM contract information.

**Parameters**:
- `address` (string) - Contract address

**Returns**: Contract object with:
- `address`: Contract address
- `creator`: Creator address
- `bytecode`: Contract bytecode
- `abi`: Contract ABI
- `contractType`: Type of contract
- `deployedAt`: Deployment timestamp
- `state`: Current contract state

---

#### `synergy_getAIVMContracts`
Get all deployed AIVM contracts.

**Parameters**:
- `filter` (object, optional) - Filter by type, creator, etc.
- `limit` (number, optional) - Maximum results

**Returns**: Array of contract objects

---

#### `synergy_getAIVMContractEvents`
Get events emitted by an AIVM contract.

**Parameters**:
- `contractAddress` (string) - Contract address
- `eventName` (string, optional) - Specific event name
- `fromBlock` (number, optional) - Starting block
- `toBlock` (number, optional) - Ending block

**Returns**: Array of event objects

---

#### `synergy_verifyAIVMContractSource`
Verify and store AIVM contract source code.

**Parameters**:
- `contractAddress` (string) - Contract address
- `sourceCode` (string) - Contract source code
- `compilerVersion` (string) - Compiler version
- `optimization` (boolean) - Optimization enabled

**Returns**: 
```json
{
  "success": true,
  "message": "Contract source verified"
}
```

---

#### `synergy_getAIVMStats`
Get AIVM runtime statistics.

**Parameters**: None

**Returns**: Stats object with:
- `totalContracts`: Total deployed contracts
- `activeContracts`: Currently active contracts
- `totalExecutions`: Total contract executions
- `gasUsed`: Total gas used by contracts
- `supportedFeatures`: List of supported features

---

### Distributed AI Methods (AIVM)

#### `synergy_initiateDistributedAI`
Initiate a distributed AI computation.

**Parameters**:
- `modelId` (string) - AI model identifier
- `inputData` (string) - Input data (hex)
- `clusterId` (number, optional) - Specific cluster
- `rewardPool` (number) - Reward pool for computation

**Returns**: 
```json
{
  "success": true,
  "computationId": "...",
  "assignedCluster": 1,
  "estimatedCompletionTime": 1640995200
}
```

---

#### `synergy_getDistributedAIStatus`
Get status of a distributed AI computation.

**Parameters**:
- `computationId` (string) - Computation ID

**Returns**: Status object with:
- `status`: Current status
- `progress`: Progress percentage
- `participatingValidators`: Array of participating validators
- `submittedPartialResults`: Number of partial results
- `estimatedCompletionTime`: Estimated completion time

---

#### `synergy_getDistributedAIResult`
Get the result of a completed distributed AI computation.

**Parameters**:
- `computationId` (string) - Computation ID

**Returns**: Result object with:
- `result`: Computation result
- `aggregationMethod`: Method used to aggregate results
- `participatingValidators`: Array of validators that contributed
- `completionTime`: Actual completion time

---

#### `synergy_submitAIPartialResult`
Submit a partial result for a distributed AI computation.

**Parameters**:
- `computationId` (string) - Computation ID
- `validatorAddress` (string) - Validator address
- `partialResult` (string) - Partial result (hex)
- `proof` (string) - Proof of correct computation

**Returns**: 
```json
{
  "success": true,
  "message": "Partial result submitted"
}
```

---

#### `synergy_getValidatorAITasks`
Get pending AI tasks for a validator.

**Parameters**:
- `validatorAddress` (string) - Validator address

**Returns**: Array of task objects

---

#### `synergy_getValidatorAIRewards`
Get AI computation rewards for a validator.

**Parameters**:
- `validatorAddress` (string) - Validator address

**Returns**: Rewards object with total and breakdown

---

#### `synergy_getAIDistributedStats`
Get distributed AI network statistics.

**Parameters**: None

**Returns**: Stats object with:
- `totalComputations`: Total AI computations
- `completedComputations`: Completed computations
- `activeComputations`: Currently active
- `totalRewardsDistributed`: Total AI rewards paid
- `averageCompletionTime`: Average completion time
- `activeValidators`: Validators participating in AI

---

#### `synergy_getAIModels`
Get available AI models for distributed computation.

**Parameters**:
- `category` (string, optional) - Model category filter

**Returns**: Array of model objects with:
- `modelId`: Model identifier
- `name`: Model name
- `description`: Model description
- `inputFormat`: Expected input format
- `outputFormat`: Output format
- `complexity`: Computational complexity
- `averageCompletionTime`: Average time to complete

---

#### `synergy_chatWithAIVM`
Interact with an AI-powered smart contract.

**Parameters**:
- `contractAddress` (string) - AI contract address
- `message` (string) - User message
- `context` (object, optional) - Conversation context

**Returns**: AI response object

---

## Priority 5: Governance & DAO

### DAO Governance Methods

#### `synergy_getProposals`
Get all governance proposals.

**Parameters**:
- `status` (string, optional) - Filter by status: "active", "passed", "rejected", "executed"
- `limit` (number, optional) - Maximum results

**Returns**: Array of proposal objects

---

#### `synergy_getProposal`
Get a specific governance proposal.

**Parameters**:
- `proposalId` (string) - Proposal ID

**Returns**: Proposal object with:
- `proposalId`: Proposal ID
- `title`: Proposal title
- `description`: Proposal description
- `proposer`: Proposer address
- `status`: Current status
- `votesFor`: Votes in favor
- `votesAgainst`: Votes against
- `votesAbstain`: Abstain votes
- `votingStartTime`: Voting start timestamp
- `votingEndTime`: Voting end timestamp
- `executionTime`: Execution timestamp (if passed)

---

#### `synergy_createProposal`
Create a new governance proposal.

**Parameters**:
- `title` (string) - Proposal title
- `description` (string) - Proposal description
- `actions` (array) - Proposed actions
- `proposer` (string) - Proposer address
- `deposit` (number) - Proposal deposit amount

**Returns**: 
```json
{
  "success": true,
  "proposalId": "...",
  "message": "Proposal created"
}
```

---

#### `synergy_voteProposal`
Vote on a governance proposal.

**Parameters**:
- `proposalId` (string) - Proposal ID
- `voter` (string) - Voter address
- `vote` (string) - Vote type: "for", "against", "abstain"
- `signature` (string) - Signed vote

**Returns**: 
```json
{
  "success": true,
  "message": "Vote recorded"
}
```

---

#### `synergy_executeProposal`
Execute a passed governance proposal.

**Parameters**:
- `proposalId` (string) - Proposal ID
- `executor` (string) - Executor address

**Returns**: 
```json
{
  "success": true,
  "message": "Proposal executed",
  "results": [...]
}
```

---

#### `synergy_getVotingPower`
Get the voting power of an address.

**Parameters**:
- `address` (string) - Address to query
- `blockTag` (string, optional) - Block tag

**Returns**: 
```json
{
  "votingPower": 1000000,
  "delegatedVotingPower": 500000,
  "totalVotingPower": 1500000
}
```

---

#### `synergy_delegateVotingPower`
Delegate voting power to another address.

**Parameters**:
- `delegator` (string) - Delegator address
- `delegate` (string) - Delegate address
- `amount` (number) - Amount to delegate (or max if not specified)

**Returns**: 
```json
{
  "success": true,
  "message": "Voting power delegated"
}
```

---

#### `synergy_getTreasuryBalance`
Get the DAO treasury balance.

**Parameters**: None

**Returns**: Treasury object with balances for all tokens

---

#### `synergy_proposeTreasurySpend`
Propose a treasury expenditure.

**Parameters**:
- `recipient` (string) - Recipient address
- `amount` (number) - Amount to spend
- `token` (string) - Token symbol
- `purpose` (string) - Purpose description
- `proposer` (string) - Proposer address

**Returns**: 
```json
{
  "success": true,
  "proposalId": "..."
}
```

---

## Priority 6: Analytics & Monitoring

### Analytics Methods

#### `synergy_getAddressTransactions`
Get all transactions for an address.

**Parameters**:
- `address` (string) - Address to query
- `fromBlock` (number, optional) - Starting block
- `toBlock` (number, optional) - Ending block
- `limit` (number, optional) - Maximum results

**Returns**: Array of transaction objects

---

#### `synergy_getAddressInternalTransactions`
Get internal transactions for an address.

**Parameters**:
- `address` (string) - Address to query
- `fromBlock` (number, optional) - Starting block
- `toBlock` (number, optional) - Ending block

**Returns**: Array of internal transaction objects

---

#### `synergy_getAddressBalanceHistory`
Get balance history for an address.

**Parameters**:
- `address` (string) - Address to query
- `fromBlock` (number, optional) - Starting block
- `toBlock` (number, optional) - Ending block

**Returns**: Array of balance snapshots

---

#### `synergy_getTokenHolders`
Get all holders of a token.

**Parameters**:
- `tokenSymbol` (string) - Token symbol
- `limit` (number, optional) - Maximum results
- `offset` (number, optional) - Offset for pagination

**Returns**: Array of holder objects with:
- `address`: Holder address
- `balance`: Token balance
- `percentage`: Percentage of total supply

---

#### `synergy_getTokenTransfers`
Get all transfers of a specific token.

**Parameters**:
- `tokenSymbol` (string) - Token symbol
- `fromBlock` (number, optional) - Starting block
- `toBlock` (number, optional) - Ending block
- `limit` (number, optional) - Maximum results

**Returns**: Array of transfer objects

---

#### `synergy_getNetworkActivity`
Get network activity statistics.

**Parameters**:
- `period` (string, optional) - Time period: "1h", "24h", "7d", "30d"

**Returns**: Activity object with:
- `totalTransactions`: Total transactions
- `uniqueAddresses`: Unique active addresses
- `averageBlockTime`: Average block time
- `averageGasPrice`: Average gas price
- `totalVolume`: Total transaction volume

---

#### `synergy_getTransactionVolume`
Get transaction volume statistics.

**Parameters**:
- `period` (string, optional) - Time period
- `token` (string, optional) - Specific token

**Returns**: Volume statistics with hourly/daily breakdown

---

#### `synergy_getActiveAddresses`
Get count of active addresses.

**Parameters**:
- `period` (string, optional) - Time period

**Returns**: Object with:
- `dailyActive`: Daily active addresses
- `weeklyActive`: Weekly active addresses
- `monthlyActive`: Monthly active addresses

---

#### `synergy_getContractInteractions`
Get contract interaction statistics.

**Parameters**:
- `contractAddress` (string) - Contract address
- `period` (string, optional) - Time period

**Returns**: Interaction statistics

---

#### `synergy_getGasTracker`
Get real-time gas tracking information.

**Parameters**: None

**Returns**: Gas tracker object with:
- `slow`: Slow gas price and estimated time
- `average`: Average gas price and estimated time
- `fast`: Fast gas price and estimated time
- `baseFee`: Current base fee

---

### Monitoring & Health

#### `synergy_getHealthCheck`
Get comprehensive node health check.

**Parameters**: None

**Returns**: Health object with:
- `status`: Overall status ("healthy", "degraded", "unhealthy")
- `blockchain`: Blockchain sync status
- `p2p`: P2P network status
- `rpc`: RPC server status
- `memory`: Memory usage
- `disk`: Disk usage
- `cpu`: CPU usage

---

#### `synergy_getMetrics`
Get Prometheus-compatible metrics.

**Parameters**: None

**Returns**: Metrics in Prometheus format

---

#### `synergy_getDebugInfo`
Get debug information for troubleshooting.

**Parameters**:
- `category` (string, optional) - Debug category

**Returns**: Debug information object

---

#### `synergy_getConfig`
Get the current node configuration.

**Parameters**: None

**Returns**: Configuration object (sensitive data redacted)

---

#### `synergy_reloadConfig`
Reload node configuration.

**Parameters**: None

**Returns**: 
```json
{
  "success": true,
  "message": "Configuration reloaded"
}
```

---

#### `synergy_getLogLevels`
Get current log levels.

**Parameters**: None

**Returns**: Object with current log levels for each module

---

#### `synergy_setLogLevel`
Set log level for a module.

**Parameters**:
- `module` (string) - Module name
- `level` (string) - Log level: "error", "warn", "info", "debug", "trace"

**Returns**: 
```json
{
  "success": true,
  "message": "Log level updated"
}
```

---

## Priority 7: Cross-Chain (SXCP Extensions)

### Cross-Chain Bridge Methods

#### `synergy_initiateBridgeTransfer`
Initiate a cross-chain bridge transfer.

**Parameters**:
- `fromAddress` (string) - Source address
- `toAddress` (string) - Destination address on target chain
- `token` (string) - Token to bridge
- `amount` (number) - Amount to bridge
- `targetChain` (string) - Target chain identifier
- `initiator` (string) - Initiator address

**Returns**: 
```json
{
  "success": true,
  "bridgeTransferId": "...",
  "estimatedCompletionTime": 1640995200,
  "fee": 1000000
}
```

---

#### `synergy_getBridgeTransferStatus`
Get status of a bridge transfer.

**Parameters**:
- `bridgeTransferId` (string) - Bridge transfer ID

**Returns**: Status object with current state and progress

---

#### `synergy_getBridgeConfig`
Get bridge configuration for a target chain.

**Parameters**:
- `targetChain` (string) - Target chain identifier

**Returns**: Bridge configuration object

---

#### `synergy_getBridgeLimits`
Get bridge transfer limits.

**Parameters**:
- `targetChain` (string) - Target chain identifier
- `token` (string) - Token symbol

**Returns**: Limits object with min/max amounts and daily limits

---

#### `synergy_getBridgeFeeEstimate`
Estimate bridge transfer fees.

**Parameters**:
- `targetChain` (string) - Target chain identifier
- `token` (string) - Token symbol
- `amount` (number) - Amount to bridge

**Returns**: Fee estimate object

---

#### `synergy_claimBridgedTokens`
Claim tokens on the destination chain.

**Parameters**:
- `bridgeTransferId` (string) - Bridge transfer ID
- `claimer` (string) - Claimer address
- `proof` (string) - Attestation proof

**Returns**: 
```json
{
  "success": true,
  "transactionHash": "..."
}
```

---

#### `synergy_getBridgedTokenBalance`
Get balance of bridged tokens.

**Parameters**:
- `address` (string) - Address to query
- `originalChain` (string) - Original chain
- `token` (string) - Token symbol

**Returns**: Bridged token balance

---

#### `synergy_getBridgeHistory`
Get bridge transfer history.

**Parameters**:
- `address` (string) - Address to query
- `direction` (string, optional) - "inbound" or "outbound"
- `limit` (number, optional) - Maximum results

**Returns**: Array of bridge transfer objects

---

### Oracle Methods

#### `synergy_submitOracleData`
Submit data from an oracle.

**Parameters**:
- `oracleAddress` (string) - Oracle address
- `dataId` (string) - Data identifier
- `value` (any) - Data value
- `timestamp` (number) - Data timestamp
- `signature` (string) - Oracle signature

**Returns**: 
```json
{
  "success": true,
  "message": "Oracle data submitted"
}
```

---

#### `synergy_getOracleData`
Get the latest oracle data.

**Parameters**:
- `dataId` (string) - Data identifier

**Returns**: Oracle data object with value and metadata

---

#### `synergy_getOracleProviders`
Get all registered oracle providers.

**Parameters**: None

**Returns**: Array of oracle provider objects

---

#### `synergy_registerOracleProvider`
Register a new oracle provider.

**Parameters**:
- `address` (string) - Oracle address
- `metadata` (object) - Oracle metadata
- `deposit` (number) - Security deposit

**Returns**: 
```json
{
  "success": true,
  "message": "Oracle provider registered"
}
```

---

#### `synergy_getPriceFeed`
Get price feed data.

**Parameters**:
- `pair` (string) - Trading pair (e.g., "SNRG/USD")

**Returns**: Price feed object with current price and history

---

## Priority 8: Developer Tools

### Debug & Testing Methods

#### `synergy_mine`
Mine a new block (testnet only).

**Parameters**:
- `count` (number, optional) - Number of blocks to mine

**Returns**: Array of mined block hashes

**Note**: Testnet/debug only - should be disabled on mainnet.

---

#### `synergy_setAccountBalance`
Set an account's balance (testnet only).

**Parameters**:
- `address` (string) - Address
- `balance` (number) - New balance

**Returns**: 
```json
{
  "success": true,
  "message": "Balance updated"
}
```

**Note**: Testnet/debug only.

---

#### `synergy_impersonateAccount`
Impersonate an account for testing.

**Parameters**:
- `address` (string) - Address to impersonate

**Returns**: 
```json
{
  "success": true,
  "message": "Account impersonated"
}
```

**Note**: Testnet/debug only.

---

#### `synergy_stopImpersonatingAccount`
Stop impersonating an account.

**Parameters**:
- `address` (string) - Address to stop impersonating

**Returns**: 
```json
{
  "success": true,
  "message": "Stopped impersonating"
}
```

---

#### `synergy_snapshot`
Take a blockchain state snapshot (testnet only).

**Parameters**: None

**Returns**: 
```json
{
  "success": true,
  "snapshotId": "..."
}
```

---

#### `synergy_revertToSnapshot`
Revert to a previous state snapshot.

**Parameters**:
- `snapshotId` (string) - Snapshot ID

**Returns**: 
```json
{
  "success": true,
  "message": "State reverted"
}
```

---

#### `synergy_reset`
Reset the blockchain to a specific state (testnet only).

**Parameters**:
- `blockNumber` (number) - Block number to reset to, OR
- `jsonConfig` (object) - Genesis configuration

**Returns**: 
```json
{
  "success": true,
  "message": "Blockchain reset"
}
```

---

#### `synergy_setStorageAt`
Set storage at a specific slot (testnet only).

**Parameters**:
- `address` (string) - Contract address
- `slot` (string) - Storage slot (hex)
- `value` (string) - Value to set (hex)

**Returns**: 
```json
{
  "success": true,
  "message": "Storage updated"
}
```

---

#### `synergy_setCode`
Set code at an address (testnet only).

**Parameters**:
- `address` (string) - Address
- `code` (string) - Contract bytecode (hex)

**Returns**: 
```json
{
  "success": true,
  "message": "Code updated"
}
```

---

#### `synergy_setNonce`
Set the nonce for an address (testnet only).

**Parameters**:
- `address` (string) - Address
- `nonce` (number) - New nonce

**Returns**: 
```json
{
  "success": true,
  "message": "Nonce updated"
}
```

---

### Batch & Multi-Call Methods

#### `synergy_batch`
Execute multiple RPC calls in a batch.

**Parameters**:
- `calls` (array) - Array of RPC call objects

**Returns**: Array of results in the same order

**Use Case**: Reduce network round trips for multiple queries.

---

#### `synergy_multicall`
Execute multiple contract calls in a single transaction.

**Parameters**:
- `calls` (array) - Array of contract call objects
- `blockTag` (string, optional) - Block tag

**Returns**: Array of call results

**Use Case**: Efficiently query multiple contract states.

---

#### `synergy_aggregate`
Aggregate multiple queries with custom logic.

**Parameters**:
- `queries` (array) - Array of queries
- `aggregation` (string) - Aggregation function

**Returns**: Aggregated result

---

## Priority 9: Security & Privacy

### Security Methods

#### `synergy_getSecurityAudit`
Get security audit information for the node.

**Parameters**: None

**Returns**: Security audit object with:
- `lastAudit`: Last audit timestamp
- `vulnerabilities`: Known vulnerabilities
- `recommendations`: Security recommendations
- `complianceStatus`: Compliance status

---

#### `synergy_rotateKeys`
Rotate validator keys.

**Parameters**:
- `validatorAddress` (string) - Validator address
- `newPublicKey` (string) - New public key
- `signature` (string) - Signed rotation request

**Returns**: 
```json
{
  "success": true,
  "message": "Keys rotated successfully"
}
```

---

#### `synergy_emergencyStop`
Initiate emergency stop procedure.

**Parameters**:
- `reason` (string) - Reason for emergency stop
- `authorizedBy` (string) - Authorizing address

**Returns**: 
```json
{
  "success": true,
  "message": "Emergency stop initiated"
}
```

---

#### `synergy_getEncryptionKey`
Get encryption key for secure communication.

**Parameters**:
- `keyType` (string) - Type of key needed

**Returns**: Encrypted key object

---

#### `synergy_verifyMessage`
Verify a signed message.

**Parameters**:
- `address` (string) - Signer address
- `message` (string) - Original message
- `signature` (string) - Signature

**Returns**: 
```json
{
  "valid": true,
  "signer": "synw..."
}
```

---

#### `synergy_signMessage`
Sign a message with a wallet's private key.

**Parameters**:
- `address` (string) - Wallet address
- `message` (string) - Message to sign

**Returns**: 
```json
{
  "success": true,
  "signature": "..."
}
```

---

### Privacy Methods

#### `synergy_getPrivateTransactionStatus`
Get status of a private transaction.

**Parameters**:
- `txHash` (string) - Transaction hash

**Returns**: Private transaction status

---

#### `synergy_getPrivacyPoolStats`
Get statistics about the privacy pool.

**Parameters**: None

**Returns**: Pool statistics

---

## Priority 10: Miscellaneous

### Utility Methods

#### `synergy_getEpochInfo`
Get current epoch information.

**Parameters**: None

**Returns**: Epoch object with:
- `epoch`: Current epoch number
- `startTime`: Epoch start timestamp
- `endTime`: Epoch end timestamp
- `totalValidators`: Validators in epoch
- `totalBlocks`: Blocks in epoch
- `totalRewards`: Total rewards distributed

---

#### `synergy_getHistoricalEpoch`
Get information about a historical epoch.

**Parameters**:
- `epoch` (number) - Epoch number

**Returns**: Historical epoch information

---

#### `synergy_getInflationRate`
Get current inflation rate.

**Parameters**: None

**Returns**: 
```json
{
  "currentInflationRate": 0.05,
  "annualInflationRate": 0.05,
  "nextAdjustmentAt": 1640995200
}
```

---

#### `synergy_getCirculatingSupply`
Get circulating token supply.

**Parameters**: None

**Returns**: 
```json
{
  "circulatingSupply": 1000000000,
  "totalSupply": 1200000000,
  "stakedSupply": 500000000,
  "lockedSupply": 100000000
}
```

---

#### `synergy_getTokenSupply`
Get token supply information.

**Parameters**:
- `token` (string, optional) - Token symbol (default: "SNRG")

**Returns**: Supply breakdown object

---

#### `synergy_getProtocolVersion`
Get the protocol version.

**Parameters**: None

**Returns**: 
```json
{
  "version": "1.0.0",
  "minCompatibleVersion": "0.9.0",
  "recommendedVersion": "1.0.0"
}
```

---

#### `synergy_getForkSchedule`
Get upcoming fork schedule.

**Parameters**: None

**Returns**: Array of scheduled forks with:
- `name`: Fork name
- `block`: Activation block
- `features`: New features
- `status`: "scheduled", "active"

---

#### `synergy_getSpec`
Get network specification parameters.

**Parameters**: None

**Returns**: Spec object with all network constants

---

#### `synergy_getConstants`
Get network constants and parameters.

**Parameters**: None

**Returns**: Object with network constants

---

### Event & Notification Methods

#### `synergy_getUpcomingEvents`
Get upcoming network events.

**Parameters**:
- `limit` (number, optional) - Maximum events to return

**Returns**: Array of event objects

---

#### `synergy_registerWebhook`
Register a webhook for event notifications.

**Parameters**:
- `url` (string) - Webhook URL
- `events` (array) - Events to subscribe to
- `secret` (string, optional) - Webhook secret

**Returns**: 
```json
{
  "success": true,
  "webhookId": "..."
}
```

---

#### `synergy_unregisterWebhook`
Unregister a webhook.

**Parameters**:
- `webhookId` (string) - Webhook ID

**Returns**: 
```json
{
  "success": true,
  "message": "Webhook unregistered"
}
```

---

## Implementation Priority Summary

### ~~Phase 1 (Critical - Implement First)~~ ✅ COMPLETED
> All 9 critical methods + 9 bonus methods implemented and documented in `rpc-methods.md`.

### ~~Phase 2 (High Priority)~~ ✅ COMPLETED
> 20 methods implemented: validator rewards/performance/queue/slashing, cluster info/rewards/proposals, staking rewards/APY/delegations/claims/projections, chain ID, unstaking period. Documented in `rpc-methods.md`.
> **Note**: `synergy_subscribe`/`synergy_unsubscribe` (WebSocket) deferred to Phase 3 as they require WebSocket transport support.

### Phase 3 (Medium Priority) ← CURRENT
1. AIVM contract methods (when AIVM is re-enabled)
2. DAO governance methods
3. Analytics and monitoring methods
4. Cross-chain bridge methods
5. Developer tools and debug methods

### Phase 4 (Lower Priority)
1. Advanced privacy methods
2. Oracle methods
3. Webhook notifications
4. Specialized analytics

---

## Notes

1. **Testnet-Only Methods**: Methods marked as testnet-only should be disabled or require special authorization on mainnet.

2. **WebSocket Support**: Subscription methods require WebSocket support in addition to HTTP RPC.

3. **Rate Limiting**: Consider implementing rate limiting for resource-intensive methods.

4. **Authentication**: Some methods may require authentication or authorization.

5. **Backward Compatibility**: Maintain backward compatibility when possible when adding new methods.

6. **Documentation**: Each new method should be documented with:
   - Clear description
   - Parameter specifications
   - Return value format
   - Example usage
   - Error cases

7. **Testing**: All new methods should have comprehensive unit and integration tests.

8. **Gas Costs**: Consider gas costs for state-modifying methods.

9. **Security Review**: All methods should undergo security review before deployment.

10. **Community Input**: Consider community feedback for method prioritization and design.
