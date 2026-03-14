# Synergy Network Staking Guide

## Overview

Staking is a core mechanism in the Synergy Network that allows token holders to participate in network security and earn rewards. This guide explains how staking works, how to participate, and best practices for stakers.

## What is Staking?

Staking involves locking tokens to support network validators in exchange for rewards. In the Synergy Network:

- **Stakers** delegate tokens to validators
- **Validators** secure the network and produce blocks
- **Rewards** are distributed based on participation and performance

## Benefits of Staking

### For Stakers
- **Earn Rewards**: Regular token rewards for participation
- **Support Network**: Help secure and decentralize the network
- **Voting Rights**: Participate in governance decisions
- **Lower Risk**: No technical requirements like running a node

### For the Network
- **Security**: Economic incentives prevent attacks
- **Decentralization**: Distributes influence across participants
- **Performance**: Encourages active validator participation

## How Staking Works

### 1. Token Locking
Stakers lock their tokens for a period, making them unavailable for transfer but earning rewards.

### 2. Validator Selection
Tokens are delegated to validators who:
- Run network nodes
- Validate transactions
- Produce new blocks
- Maintain network security

### 3. Reward Distribution
Rewards are calculated and distributed based on:
- Amount staked
- Staking duration
- Validator performance
- Network parameters

### 4. Unstaking Process
Stakers can unlock tokens after a cooldown period.

## Getting Started

### Prerequisites
- SNRG tokens in your wallet
- Understanding of validator selection
- Awareness of risks and rewards

### Step 1: Choose a Validator

Research validators before staking:

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTopValidators",
  "params": [20],
  "id": 1
}
```

Consider factors:
- **Uptime**: Historical reliability
- **Commission**: Fee charged by validator
- **Stake Amount**: Total tokens delegated
- **Performance**: Block production and validation

### Step 2: Stake Tokens

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_stakeTokensDirect",
  "params": ["sYn...", "sYn...", "SNRG", 1000000],
  "id": 1
}
```

Parameters:
- `staker`: Your wallet address
- `validator`: Chosen validator address
- `token_symbol`: "SNRG" for native token
- `amount`: Amount to stake

### Step 3: Monitor Your Stake

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

## Staking Parameters

### Minimum Stake
- **SNRG**: 1,000 tokens minimum
- Can be changed via governance

### Lock-up Period
- **Staking**: Immediate effect
- **Unstaking**: 7-day cooldown period
- **Rewards**: Distributed daily

### Reward Rates
- **Base Rate**: 5-15% APY (variable)
- **Performance Bonus**: Up to 50% additional
- **Penalties**: Applied for validator misbehavior

## Validator Responsibilities

### Node Operation
- Maintain 99.9% uptime
- Follow network upgrade schedules
- Participate in consensus mechanisms

### Security
- Protect private keys
- Monitor for attacks
- Report suspicious activity

### Transparency
- Publish performance metrics
- Communicate with delegators
- Participate in governance

## Risks and Considerations

### Slashing Risks
Validators can be penalized for:
- **Double Signing**: Signing conflicting blocks
- **Downtime**: Extended periods offline
- **Malicious Behavior**: Attacking the network

### Market Risks
- **Token Volatility**: SNRG price fluctuations
- **Impermanent Loss**: Opportunity cost of staking
- **Liquidity**: Locked tokens unavailable for trading

### Technical Risks
- **Validator Issues**: Hardware/software failures
- **Network Upgrades**: Potential downtime
- **Smart Contract Bugs**: Unforeseen vulnerabilities

## Advanced Staking Strategies

### Diversification
- Stake across multiple validators
- Balance risk and reward
- Monitor validator performance

### Re-staking Rewards
- Compound rewards automatically
- Increase staking position over time
- Consider tax implications

### Active Participation
- Vote in governance proposals
- Monitor validator performance
- Switch validators if needed

## Unstaking Process

### Request Unstaking

```json
{
  "jsonrpc": "2.0",
  "method": "synergy_unstakeTokens",
  "params": ["sYn...", "sYn...", "SNRG", 500000],
  "id": 1
}
```

### Cooldown Period
- **Duration**: 7 days
- **Status**: Tokens remain locked but not earning rewards
- **Monitoring**: Track progress via API

### Claim Tokens
- Tokens become available after cooldown
- Check balance and transfer as needed

## Staking Analytics

### Performance Metrics
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getValidatorStats",
  "params": [],
  "id": 1
}
```

### Reward Calculations
- **Daily Rewards**: (Staked Amount × Annual Rate) ÷ 365
- **Monthly Rewards**: Daily Rewards × 30
- **APY**: ((1 + Daily Rate)^365 - 1) × 100

### Network Statistics
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getNetworkStats",
  "params": [],
  "id": 1
}
```

## Troubleshooting

### Common Issues

**"Insufficient balance"**:
- Check available SNRG balance
- Verify token symbol is "SNRG"
- Consider transaction fees

**"Validator not found"**:
- Verify validator address
- Check if validator is active
- Ensure validator accepts delegations

**"Stake too low"**:
- Meet minimum stake requirements
- Check current minimum stake amount
- Consider staking more tokens

### Getting Help

1. **Check Documentation**: Review this guide
2. **API Reference**: Verify parameter formats
3. **Community**: Ask questions in forums
4. **Support**: Contact technical support

## Tax Implications

### Reward Taxation
- Staking rewards may be taxable
- Track all reward distributions
- Consult local tax regulations

### Record Keeping
- Maintain transaction records
- Track staking duration and amounts
- Document validator performance

## Future Developments

### Liquid Staking
- Stake tokens while maintaining liquidity
- Trade staked positions
- Enhanced DeFi integration

### Staking Pools
- Pool tokens with other stakers
- Share validator selection
- Reduce individual risk

### Cross-chain Staking
- Stake on multiple networks
- Unified reward management
- Enhanced interoperability

## Best Practices

### Security
1. Research validators thoroughly
2. Diversify across multiple validators
3. Monitor validator performance regularly
4. Keep software and keys secure

### Strategy
1. Start with small amounts to learn
2. Re-stake rewards to compound returns
3. Stay informed about network changes
4. Participate in governance

### Risk Management
1. Only stake what you can afford to lock
2. Understand slashing conditions
3. Monitor validator communications
4. Have an exit strategy

---

*This staking guide provides general information and should not be considered financial advice. Always do your own research and understand the risks involved in staking.*
