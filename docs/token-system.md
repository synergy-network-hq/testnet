# Synergy Network Token System

## Overview

The Synergy Network features a comprehensive token system that supports multiple token types, staking, and advanced token operations. The system is designed to be flexible and extensible while maintaining security and performance.

## Token Structure

### Token Properties

Each token in the Synergy Network has the following properties:

- **Symbol**: Unique identifier (e.g., "SNRG", "USDS")
- **Name**: Human-readable name (e.g., "Synergy Coin", "Synergy USD")
- **Decimals**: Number of decimal places (9 is Synergy Network default)
- **Total Supply**: Current circulating supply
- **Max Supply**: Maximum possible supply (optional)
- **Mintable**: Whether new tokens can be created
- **Burnable**: Whether tokens can be destroyed
- **Creator**: Address that created the token

### Native Token: SNRG

The Synergy Network has a native token called Synergy Coin (SNRG):

- **Symbol**: SNRG
- **Name**: SynergyCoin
- **Decimals**: 9
- **Max Supply**: 12,000,000,000 SNRG (12 billion)
- **Mintable**: No
- **Burnable**: Yes

## Token Operations

### Creating Tokens

Users can create new tokens on the network:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_createToken",
  "params": ["MYTOKEN", "My Token", 18, 1000000, "sYn..."],
  "id": 1
}
```

**Parameters:**

- `symbol`: Token symbol (unique)
- `name`: Token name
- `decimals`: Number of decimal places
- `total_supply`: Initial supply
- `creator`: Creator address

### Minting Tokens

Authorized users can mint new tokens:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_mintTokens",
  "params": ["sYn...", "SNRG", 1000000],
  "id": 1
}
```

**Parameters:**

- `to`: Recipient address
- `token_symbol`: Token to mint
- `amount`: Amount to mint

### Burning Tokens

Token holders can burn their tokens:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_burnTokens",
  "params": ["sYn...", "SNRG", 1000000],
  "id": 1
}
```

**Parameters:**

- `from`: Address burning tokens
- `token_symbol`: Token to burn
- `amount`: Amount to burn

### Transferring Tokens

Users can transfer tokens between addresses:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_transferTokens",
  "params": ["sYn...", "sYn...", "SNRG", 1000],
  "id": 1
}
```

**Parameters:**

- `from`: Sender address
- `to`: Recipient address
- `token_symbol`: Token to transfer
- `amount`: Amount to transfer

## Balance Management

### Checking Balances

Query token balances for an address:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTokenBalance",
  "params": ["sYn...", "SNRG"],
  "id": 1
}
```

Returns:

```json
{
  "balance": 1000000
}
```

### All Balances

Get all token balances for an address:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getAllBalances",
  "params": ["sYn..."],
  "id": 1
}
```

Returns:

```json
{
  "SNRG": 1000000,
  "USDS": 50000
}
```

## Staking System

### Staking Tokens

Stake tokens to validators to earn rewards:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_stakeTokensDirect",
  "params": ["sYn...", "sYn...", "SNRG", 1000000],
  "id": 1
}
```

**Parameters:**

- `staker`: Staker address
- `validator`: Validator address
- `token_symbol`: Token to stake
- `amount`: Amount to stake

### Unstaking Tokens

Withdraw staked tokens:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_unstakeTokens",
  "params": ["sYn...", "sYn...", "SNRG", 500000],
  "id": 1
}
```

**Parameters:**

- `staker`: Staker address
- `validator`: Validator address
- `token_symbol`: Token to unstake
- `amount`: Amount to unstake

### Staking Information

Query staking information:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getStakingInfo",
  "params": ["sYn..."],
  "id": 1
}
```

Returns:

```json
[
  {
    "validator_address": "sYn...",
    "staker_address": "sYn...",
    "amount": 1000000,
    "stake_start": 1640995200,
    "stake_end": null,
    "rewards_earned": 50000,
    "is_active": true
  }
]
```

## Transfer History

### Transaction History

Get transfer history for an address:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTransferHistory",
  "params": ["sYn...", 50],
  "id": 1
}
```

Returns:

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

## Token Statistics

### Token Information

Get detailed information about all tokens:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTokens",
  "params": [],
  "id": 1
}
```

Returns:

```json
[
  {
    "symbol": "SNRG",
    "name": "Synergy Coin",
    "decimals": 9,
    "total_supply": 12000000000000000000000000000,
    "max_supply": 12000000000000000000000000000,
    "mintable": false,
    "burnable": true,
    "created_at": 1640995200,
    "creator": "genesis"
  }
]
```

### Token Statistics

Get comprehensive token statistics:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTokenStats",
  "params": [],
  "id": 1
}
```

Returns:

```json
[
  {
    "symbol": "SNRG",
    "name": "Synergy Coin",
    "total_supply": 12000000000000000000000000000,
    "total_staked": 500000000000000000000,
    "holders": 25
  }
]
```

## Security Features

### Balance Validation

All token operations include balance validation to prevent:

- Insufficient funds errors
- Double-spending
- Invalid token operations

### Access Control

- Only authorized addresses can mint tokens
- Burn operations require sufficient balance
- Transfer operations validate sender balance

### State Management

Token state is managed through:

- Persistent storage across node restarts
- Atomic operations to prevent corruption
- Rollback capabilities for failed operations

## Use Cases

### DeFi Applications

The token system supports:

- Creating custom tokens for DApps
- Staking mechanisms for yield farming
- Token swaps and liquidity pools

### Gaming and NFTs

- In-game currencies
- NFT marketplaces
- Gaming rewards and achievements

### Enterprise Solutions

- Corporate tokens
- Supply chain tracking
- Digital asset management

## Best Practices

### Token Creation

1. Choose appropriate decimal places (18 is standard)
2. Set reasonable max supply limits
3. Consider making tokens mintable for future expansion
4. Use descriptive names and symbols

### Staking

1. Research validators before staking
2. Consider lock-up periods
3. Monitor reward distribution
4. Diversify across multiple validators

### Security

1. Never share private keys
2. Verify all transaction details before signing
3. Use hardware wallets for large amounts
4. Keep software updated

## Troubleshooting

### Common Issues

**"Insufficient balance"**: Ensure you have enough tokens for the operation
**"Token not found"**: Verify the token symbol is correct
**"Unauthorized"**: Check if you have permission for the operation
**"Network error"**: Verify node connectivity

### Getting Help

For technical support:

- Check the troubleshooting guide
- Review API documentation
- Contact the development team

## Future Enhancements

Planned features for the token system:

- Cross-chain token transfers
- Advanced staking mechanisms
- Token governance features
- Automated market makers
- Layer 2 scaling solutions
