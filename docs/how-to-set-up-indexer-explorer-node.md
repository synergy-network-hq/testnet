# How to set up an Indexer & Explorer node

This guide sets up a real `Indexer & Explorer Node` that can index live chain data, expose the Atlas API, and optionally serve the web explorer UI. It is written for the current repo layout and runtime behavior in `synergy-testnet-beta`.

This guide assumes a dedicated Linux host. The role runtime is cross-platform, but the checked-in deployment assets for the explorer UI are Linux/PM2/nginx oriented.

## What you need before you start

- A machine with:
  - Ubuntu 22.04 or newer
  - 4+ CPU cores
  - 8+ GB RAM
  - 100+ GB free disk if you want to retain substantial indexed history
- A working Testnet-Beta network:
  - At least 3 validator nodes online and producing blocks
  - Seed services redeployed on the build that supports `/peers/register`
- A repo layout that matches what the runtime expects:
  - `synergy-testnet-beta` checked out locally
  - `explorer-app` either inside that repo as `explorer-app/` or next to it as a sibling directory
- Node.js available on `PATH`
- PostgreSQL running locally or reachable over the network
- A `DATABASE_URL` value you can export before starting the role node

Today, the safest setup path is a repo checkout plus a repo-built `synergy-indexer-and-explorer-node` binary. The packaged app bundle is not the right entry point for this role yet because the role runtime looks for the repo root and a repo-relative `explorer-app` directory.

## Expected directory layout

Use one of these two layouts:

```text
/opt/synergy/
├── synergy-testnet-beta/
└── explorer-app/
```

or:

```text
/opt/synergy/synergy-testnet-beta/
└── explorer-app/
```

In your current workspace, the matching layout is:

```text
/Users/devpup/Desktop/Testnet-Beta/
├── synergy-testnet-beta/
└── explorer-app/
```

## 1. Install system dependencies

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  git \
  nginx \
  postgresql \
  postgresql-contrib
```

Install Node.js 20:

```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
node --version
npm --version
```

Install Rust if the host does not already have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Install PM2 if you want the web UI and API managed outside the role node lifecycle:

```bash
sudo npm install -g pm2
pm2 --version
```

## 2. Place the repos in the expected layout

Example:

```bash
sudo mkdir -p /opt/synergy
sudo chown -R "$USER":"$USER" /opt/synergy

cd /opt/synergy
git clone <your-synergy-testnet-beta-repo-url> synergy-testnet-beta
git clone <your-explorer-app-repo-url> explorer-app
```

If you already have the repos, just ensure the final layout matches one of the two layouts above.

## 3. Build the Indexer & Explorer role binary

```bash
cd /opt/synergy/synergy-testnet-beta
cargo build --release --bin synergy-indexer-and-explorer-node
```

Verify the binary exists:

```bash
ls -l /opt/synergy/synergy-testnet-beta/target/release/synergy-indexer-and-explorer-node
```

## 4. Create the Postgres database

Create a dedicated database and user:

```bash
sudo -u postgres psql <<'SQL'
CREATE USER synergy WITH PASSWORD 'change-this-password';
CREATE DATABASE synergy_explorer OWNER synergy;
GRANT ALL PRIVILEGES ON DATABASE synergy_explorer TO synergy;
SQL
```

Set the connection string you will use everywhere:

```bash
export DATABASE_URL='postgres://synergy:change-this-password@127.0.0.1:5432/synergy_explorer'
```

Important:
- The Rust role runtime checks `DATABASE_URL` before it even attempts to start the Atlas backend and Atlas indexer.
- A `.env` file inside `explorer-app/backend` or `explorer-app/indexer` is not enough by itself if you start the role node through the Rust runtime.

## 5. Build the explorer UI, backend, and indexer

Build the web UI:

```bash
cd /opt/synergy/explorer-app
npm ci
npm run build
```

Build the API:

```bash
cd /opt/synergy/explorer-app/backend
npm ci
npm run build
```

Build the indexer:

```bash
cd /opt/synergy/explorer-app/indexer
npm ci
npm run build
```

Verify the runtime entrypoints exist:

```bash
ls -l /opt/synergy/explorer-app/backend/dist/index.js
ls -l /opt/synergy/explorer-app/indexer/dist/index.js
```

## 6. Create the explorer `.env` files

Create the backend env file:

```bash
cd /opt/synergy/explorer-app
cp backend/.env.example backend/.env
```

Edit `backend/.env` so it looks like this:

```dotenv
NODE_ENV=production
HOST=127.0.0.1
PORT=3020

DATABASE_URL=postgres://synergy:change-this-password@127.0.0.1:5432/synergy_explorer

SYNERGY_ENV=testnet
SYNERGY_CORE_RPC_URL=http://127.0.0.1:48638
SYNERGY_CORE_RPC_FALLBACK_URL=http://<archive-or-rpc-gateway-host>:<port>

CORS_ORIGIN=https://<your-explorer-domain>
```

Create the indexer env file:

```bash
cp indexer/.env.example indexer/.env
```

Edit `indexer/.env` so it looks like this:

```dotenv
NODE_ENV=production

DATABASE_URL=postgres://synergy:change-this-password@127.0.0.1:5432/synergy_explorer

SYNERGY_ENV=testnet
SYNERGY_CORE_RPC_URL=http://127.0.0.1:48638
SYNERGY_CORE_RPC_FALLBACK_URL=http://<archive-or-rpc-gateway-host>:<port>

START_BLOCK=
POLL_INTERVAL_MS=2000
BATCH_SIZE=50
CONFIRMATIONS=0
REORG_LOOKBACK=12
LOG_LEVEL=info
```

Notes:
- When the backend and indexer are started by the role runtime, `SYNERGY_CORE_RPC_URL` is injected automatically and points at the local role node RPC.
- Keeping the local RPC URL in the `.env` files still makes the setup understandable and also supports manual standalone runs.
- `SYNERGY_CORE_RPC_FALLBACK_URL` is strongly recommended if you have an archive validator or dedicated RPC gateway.

## 7. Run the database migrations

Run backend migrations:

```bash
cd /opt/synergy/explorer-app/backend
DATABASE_URL="$DATABASE_URL" npm run migrate
```

Run indexer migrations:

```bash
cd /opt/synergy/explorer-app/indexer
DATABASE_URL="$DATABASE_URL" npm run migrate
```

Both migration sets are expected to succeed against the same `synergy_explorer` database.

## 8. Provision the Indexer & Explorer workspace

Use the control panel to provision an `Indexer & Explorer Node` workspace.

The result should be a workspace with at least:

```text
<workspace>/
├── config/node.toml
├── config/peers.toml
├── config/aegis.toml
├── data/
├── keys/
├── logs/
└── manifests/bootstrap.json
```

If you already have a provisioned Indexer & Explorer workspace, reuse it.

## 9. Start the role node

Export `DATABASE_URL` in the shell that will start the node:

```bash
export DATABASE_URL='postgres://synergy:change-this-password@127.0.0.1:5432/synergy_explorer'
```

Start the role node from the provisioned workspace:

```bash
cd /path/to/indexer-workspace
/opt/synergy/synergy-testnet-beta/target/release/synergy-indexer-and-explorer-node \
  start \
  --config /path/to/indexer-workspace/config/node.toml
```

What this does:
- Starts the role-bound Testnet-Beta node itself
- Exposes the local node RPC
- Runs the Atlas backend migrations if needed
- Runs the Atlas indexer migrations if needed
- Starts the Atlas backend with `node backend/dist/index.js`
- Starts the Atlas indexer with `node indexer/dist/index.js`

Expected runtime files:

```text
/path/to/indexer-workspace/data/role-runtime.json
/path/to/indexer-workspace/data/logs/atlas-backend.out
/path/to/indexer-workspace/data/logs/atlas-backend.err
/path/to/indexer-workspace/data/logs/atlas-indexer.out
/path/to/indexer-workspace/data/logs/atlas-indexer.err
```

## 10. Verify the node, API, and indexer

Check the role node is running:

```bash
cat /path/to/indexer-workspace/data/synergy-testbeta.pid
```

Check the Atlas backend health endpoint:

```bash
curl http://127.0.0.1:3020/healthz
```

Expected result:

```json
{"ok":true}
```

Check readiness:

```bash
curl -i http://127.0.0.1:3020/readyz
```

You want:
- HTTP `200`
- `lagBlocks` small
- `snapshotAgeSeconds` small

Check live indexed data:

```bash
curl http://127.0.0.1:3020/api/v1/network/summary
curl http://127.0.0.1:3020/api/v1/blocks?limit=5
curl http://127.0.0.1:3020/api/v1/validators?limit=10
```

Check relayer and SXCP status:

```bash
curl http://127.0.0.1:3020/relayers/health
curl http://127.0.0.1:3020/sxcp/status
```

Watch the logs:

```bash
tail -f /path/to/indexer-workspace/data/logs/atlas-indexer.out
tail -f /path/to/indexer-workspace/data/logs/atlas-backend.out
tail -f /path/to/indexer-workspace/logs/synergy-testbeta.log
```

Signs that it is working:
- Blocks are increasing in `/api/v1/network/summary`
- `/api/v1/blocks` returns recent blocks
- `/api/v1/validators` returns validator rows
- `/relayers/health` returns quorum and online/eligible relayers
- `/sxcp/status` returns pending and finalized event counts

## 11. Serve the web explorer UI

If you only need the API for the control panel or for direct HTTP use, you can stop here.

If you also want the browser UI:

1. Build the frontend:

```bash
cd /opt/synergy/explorer-app
npm ci
npm run build
```

2. Copy the checked-in nginx template and update the paths/domain:

```bash
sudo cp /opt/synergy/explorer-app/ops/nginx/devnet-explorer.conf \
  /etc/nginx/sites-available/testnet-beta-explorer
```

3. Edit the copied config:
- Change `server_name`
- Change `root` to your actual `explorer-app/dist`
- Keep the upstream pointed at `127.0.0.1:3020`

4. Enable the site:

```bash
sudo ln -sf /etc/nginx/sites-available/testnet-beta-explorer /etc/nginx/sites-enabled/testnet-beta-explorer
sudo nginx -t
sudo systemctl reload nginx
```

5. Add TLS with certbot if the site is public.

The frontend defaults to `/api/v1`, so nginx should proxy `/api/` to the backend.

## Verify it worked

The setup is complete when all of these are true:

- The Indexer & Explorer role node stays up
- `http://127.0.0.1:3020/healthz` returns `{"ok":true}`
- `http://127.0.0.1:3020/readyz` returns HTTP `200`
- `http://127.0.0.1:3020/api/v1/blocks?limit=5` returns recent blocks
- `http://127.0.0.1:3020/api/v1/validators?limit=10` returns validator rows
- `http://127.0.0.1:3020/relayers/health` returns live relayer health
- `http://127.0.0.1:3020/sxcp/status` returns live SXCP status
- The browser UI loads and shows current block height, transactions, and validators

## Troubleshooting

### The role node starts, but the Atlas backend or Atlas indexer never appears

Check:

```bash
tail -n 100 /path/to/indexer-workspace/data/logs/atlas-backend.err
tail -n 100 /path/to/indexer-workspace/data/logs/atlas-indexer.err
```

Common causes:
- `DATABASE_URL` was not exported before starting the role node
- `node` is not on `PATH`
- `explorer-app/backend/dist/index.js` is missing
- `explorer-app/indexer/dist/index.js` is missing
- `explorer-app` is not in a repo-relative location the runtime can discover

### `/healthz` is up, but `/readyz` returns `503`

That means the backend is alive, but the indexer is behind or stale.

Check:

```bash
tail -n 100 /path/to/indexer-workspace/data/logs/atlas-indexer.out
tail -n 100 /path/to/indexer-workspace/data/logs/atlas-indexer.err
```

Common causes:
- Fewer than 3 validators are online, so the chain is not producing blocks
- The local explorer node has not synced yet
- The configured RPC fallback is wrong or unreachable

### The database is reachable, but the explorer stays empty

Check whether blocks are being indexed:

```bash
psql "$DATABASE_URL" -c 'SELECT COUNT(*) FROM blocks;'
psql "$DATABASE_URL" -c 'SELECT COUNT(*) FROM validators_current;'
psql "$DATABASE_URL" -c 'SELECT * FROM network_snapshots ORDER BY indexed_at DESC LIMIT 5;'
```

If the counts stay at zero, the indexer is not ingesting from RPC.

### The web UI loads, but API calls fail

Check:

```bash
curl http://127.0.0.1:3020/api/v1/network/summary
sudo nginx -t
```

Common causes:
- nginx is not proxying `/api/` to `127.0.0.1:3020`
- `CORS_ORIGIN` does not match the deployed UI domain
- the backend is only bound to localhost and you are bypassing nginx

## Next steps

- Put the role node under `systemd` if you want it to survive reboots
- Point a stable public domain at the nginx-served frontend
- Add a real archive or RPC gateway as `SYNERGY_CORE_RPC_FALLBACK_URL`
- Monitor `/readyz` and alert on sustained lag or stale snapshots
