Synergy Public Testnet Bootstrap Bundles

These bundles are pinned to Synergy Testnet chain ID 1264.

Canonical genesis hash:
dd9ad8cfc74be1ab17a0a0fce9db65281df1b325fe5a2530130dce8935e450b8

Network magic bytes:
d5d5bb99

Required bundle rule:
- Every node bundle must carry the exact same config/genesis.json as the repository canonical config/genesis.json.
- Any old chain 1262 data must be wiped before a node is started on chain 1264.
- Bootnodes use P2P 5620 and discovery 5680.
- Seed services use P2P 5621 and discovery 5681.
- Validator, relayer, observer, and indexer nodes use P2P 5622, qRPC 5640, WS 5660, discovery 5680, metrics 6030.
- RPC Gateway uses P2P 5623, qRPC 5641, WS 5661, discovery 5681, metrics 6031.
