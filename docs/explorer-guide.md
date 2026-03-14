# Synergy Network Block Explorer Guide

## Overview

The Synergy Network Block Explorer provides comprehensive tools for viewing blockchain data, tracking transactions, monitoring validators, and analyzing network performance. This guide explains how to use the explorer effectively.

## Accessing the Explorer

### API Access
All explorer data is available through the JSON-RPC API:

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "synergy_getLatestBlock", "params": [], "id": 1}'
```

### Web Interface
A web-based explorer interface is available at:
```
http://localhost:3000
```

## Core Features

### Block Information

#### Latest Block
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getLatestBlock",
  "params": [],
  "id": 1
}
```

Returns:
```json
{
  "block_index": 12345,
  "timestamp": 1640995200,
  "transactions": [...],
  "validator": "sYn...",
  "hash": "0x...",
  "parent_hash": "0x...",
  "gas_used": 150000,
  "gas_limit": 30000000
}
```

#### Block by Number
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getBlockByNumber",
  "params": [12345],
  "id": 1
}
```

#### Block Range
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getBlockRange",
  "params": [10000, 10100],
  "id": 1
}
```

### Transaction Data

#### Transaction by Hash
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTransactionByHash",
  "params": ["0x..."],
  "id": 1
}
```

Returns:
```json
{
  "sender": "sYn...",
  "receiver": "sYn...",
  "amount": 1000000,
  "nonce": 5,
  "gas_price": 1000,
  "gas_limit": 21000,
  "signature": "...",
  "data": "...",
  "timestamp": 1640995200,
  "block_number": 12345,
  "transaction_index": 2,
  "status": "confirmed"
}
```

#### Transactions in Block
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTransactionsInBlock",
  "params": [12345],
  "id": 1
}
```

#### Transaction Pool
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTransactionPool",
  "params": [],
  "id": 1
}
```

### Address Information

#### Address Balance
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getAllBalances",
  "params": ["sYn..."],
  "id": 1
}
```

#### Address Transactions
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTransferHistory",
  "params": ["sYn...", 100],
  "id": 1
}
```

### Validator Information

#### All Validators
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getValidators",
  "params": [],
  "id": 1
}
```

Returns:
```json
[
  {
    "address": "sYn...",
    "public_key": "...",
    "name": "Validator Name",
    "website": "https://...",
    "description": "Validator description",
    "registered_at": 1640995200,
    "last_active": 1640995200,
    "total_blocks_produced": 1500,
    "uptime_percentage": 99.9,
    "synergy_score": 85.5,
    "stake_amount": 1000000000000000000000000,
    "status": "Active"
  }
]
```

#### Validator Details
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getValidator",
  "params": ["sYn..."],
  "id": 1
}
```

#### Top Validators
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTopValidators",
  "params": [10],
  "id": 1
}
```

### Network Statistics

#### Comprehensive Stats
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getNetworkStats",
  "params": [],
  "id": 1
}
```

Returns:
```json
{
  "block_height": 12345,
  "total_transactions": 56789,
  "active_validators": 50,
  "total_supply": 1000000000000000000000000000,
  "tokens": 5,
  "network_uptime": "99.9%",
  "current_epoch": 123,
  "total_staked": 500000000000000000000000000
}
```

#### Validator Statistics
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getValidatorStats",
  "params": [],
  "id": 1
}
```

#### Token Statistics
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTokenStats",
  "params": [],
  "id": 1
}
```

## Advanced Analytics

### Performance Metrics

#### Validator Performance
- **Uptime**: Percentage of time online
- **Blocks Produced**: Total blocks validated
- **Synergy Score**: Overall performance rating
- **Commission Rate**: Fee charged to delegators

#### Network Performance
- **Block Time**: Average time between blocks
- **Transaction Throughput**: Transactions per second
- **Gas Usage**: Network utilization
- **Finality Time**: Time to confirm transactions

### Economic Data

#### Token Economics
- **Market Cap**: Total value of circulating tokens
- **Staking Ratio**: Percentage of tokens staked
- **Reward Distribution**: How rewards are allocated
- **Validator Economics**: Revenue and costs for validators

#### Supply Metrics
- **Circulating Supply**: Tokens in circulation
- **Total Supply**: Maximum possible tokens
- **Inflation Rate**: Annual token creation rate

## Search and Filtering

### Search by Address
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getAllBalances",
  "params": ["sYn..."],
  "id": 1
}
```

### Search by Transaction Hash
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getTransactionByHash",
  "params": ["0x..."],
  "id": 1
}
```

### Search by Block Number
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getBlockByNumber",
  "params": [12345],
  "id": 1
}
```

### Filter by Date Range
```json
{
  "jsonrpc": "2.0",
  "method": "synergy_getBlockRange",
  "params": [10000, 15000],
  "id": 1
}
```

## Data Export

### JSON Export
All API responses can be saved as JSON files for analysis.

### CSV Export
Convert API responses to CSV for spreadsheet analysis:

```python
import json
import csv

def export_to_csv(data, filename):
    with open(filename, 'w', newline='') as csvfile:
        if data:
            fieldnames = data[0].keys()
            writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(data)
```

### Database Integration
Store explorer data in databases for custom analysis:

```sql
CREATE TABLE blocks (
    block_index INTEGER PRIMARY KEY,
    timestamp INTEGER,
    validator TEXT,
    transaction_count INTEGER,
    gas_used INTEGER
);
```

## Real-time Monitoring

### WebSocket Connections
Subscribe to real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8546');

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('New block:', data);
};
```

### Event Subscriptions
- **New Blocks**: Subscribe to block production
- **New Transactions**: Monitor transaction pool
- **Validator Updates**: Track validator status changes
- **Token Transfers**: Monitor token movements

## Analytics Tools

### Built-in Charts
- Block production over time
- Transaction volume trends
- Validator performance comparisons
- Token distribution analysis

### Custom Dashboards
Create custom analytics dashboards:

```json
{
  "dashboard": {
    "title": "Network Health",
    "widgets": [
      {
        "type": "chart",
        "data": "synergy_getNetworkStats",
        "metric": "block_height"
      },
      {
        "type": "table",
        "data": "synergy_getTopValidators",
        "columns": ["name", "synergy_score", "uptime_percentage"]
      }
    ]
  }
}
```

## API Rate Limits

### Free Tier
- 100 requests per minute
- 1,000 requests per hour
- 10,000 requests per day

### Pro Tier
- 1,000 requests per minute
- 100,000 requests per hour
- Unlimited daily requests

### Rate Limit Headers
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640995260
```

## Troubleshooting

### Common Issues

**"Block not found"**:
- Verify block number exists
- Check network synchronization
- Wait for block confirmation

**"Transaction not found"**:
- Verify transaction hash
- Check if transaction is confirmed
- Search in transaction pool

**"Rate limit exceeded"**:
- Reduce request frequency
- Upgrade to Pro tier
- Implement request caching

### Performance Tips

1. **Cache Results**: Store frequently accessed data
2. **Batch Requests**: Combine multiple queries
3. **Use Pagination**: For large result sets
4. **WebSocket**: For real-time data

## Developer Integration

### REST API Wrapper
```python
class SynergyExplorer:
    def __init__(self, endpoint='http://localhost:8545'):
        self.endpoint = endpoint

    def get_latest_block(self):
        return self._call('synergy_getLatestBlock')

    def _call(self, method, params=[]):
        payload = {
            'jsonrpc': '2.0',
            'method': method,
            'params': params,
            'id': 1
        }
        response = requests.post(self.endpoint, json=payload)
        return response.json()['result']
```

### JavaScript SDK
```javascript
class SynergyExplorer {
  constructor(endpoint = 'http://localhost:8545') {
    this.endpoint = endpoint;
  }

  async getLatestBlock() {
    const response = await fetch(this.endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'synergy_getLatestBlock',
        params: [],
        id: 1
      })
    });
    const data = await response.json();
    return data.result;
  }
}
```

## Mobile Access

### Mobile API
Optimized endpoints for mobile devices:
- Compressed responses
- Reduced data fields
- Mobile-specific formatting

### Progressive Web App
Access explorer on mobile devices:
- Offline functionality
- Push notifications
- Mobile-optimized interface

## Privacy and Security

### Data Privacy
- No personal information stored
- All data is public blockchain data
- No tracking or analytics

### API Security
- HTTPS encryption
- Rate limiting protection
- Input validation
- DDoS protection

## Support

### Documentation
- [API Reference](./api-reference.md)
- [Token System](./token-system.md)
- [Wallet Usage](./wallet-usage.md)
- [Staking Guide](./staking-guide.md)

### Community
- Developer Forum: [Link]
- Discord: [Link]
- GitHub Issues: [Link]

### Contact
- Email: explorer@synergynetwork.io
- Support Hours: 24/7
- Response Time: < 24 hours

## Future Enhancements

### Planned Features
- **Advanced Analytics**: Machine learning insights
- **Custom Alerts**: Notification system
- **API Marketplace**: Third-party integrations
- **Multi-language Support**: Internationalization
- **Dark Mode**: Enhanced user experience

### Performance Improvements
- **CDN Integration**: Faster global access
- **Database Optimization**: Improved query performance
- **Caching Layer**: Redis integration
- **Load Balancing**: Multi-server deployment

---

*This explorer guide is continuously updated to reflect the latest features and improvements.*
