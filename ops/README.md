## Testnet-Beta Ops Canonicals

This directory holds the source-of-truth operational configs that have to stay
aligned with the live Testnet-Beta infrastructure.

- `observability/prometheus.observer.yml`
  Canonical Prometheus config for the observer host.
- `nginx/testbeta-core-rpc.synergy-network.io.conf`
  Canonical public RPC / WS reverse proxy config, including allowlisted metrics
  paths for the observer.
- `nginx/testbeta-explorer.conf`
  Canonical public explorer / Atlas / indexer reverse proxy config, including
  the allowlisted explorer metrics path for the observer.

Notes:

- Validators and relayers are scraped directly on the private network plane.
- The shared public service host (`74.208.227.23`) is scraped through HTTPS
  metrics paths on `443` with an observer IP allowlist.
- Bootnode raw metrics ports are not reachable from the observer's public
  network path, so the observer monitors those roles through blackbox TCP
  probes against their public bootstrap and seed ports.
