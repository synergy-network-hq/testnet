# AIVM & GPT-OSS Quick Commands

## 1. AIVM Testnet Workflow

- **Start the Testnet with AIVM enabled**  
  ```sh
  cargo run --release --bin synergy-testnet -- start
  ```
  The runtime reads `config/aivm-config.toml`, so confirm `[aivm].enabled = true` and the GPT-OSS endpoint points to `http://localhost:8000`.

- **Deploy or execute AIVM contracts**  
  Send RPC payloads against the node (see `docs/aivm-guide.md` for JSON templates). For example:
  ```sh
  curl -s http://localhost:9933 \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"synergy_deployAIVMContract","params":[...],"id":1}'
  ```
  Replace `params` with your compiled bytecode, ABI, and `"ai"` flag to register it as an AI contract.

- **Inspect AIVM stats**  
  ```sh
  curl -s http://localhost:9933 -d '{"jsonrpc":"2.0","method":"synergy_getAIVMStats","params":[],"id":1}'
  ```
  Use the response to confirm distributed computation health and gas usage.

## 2. GPT-OSS Server Commands

- **Ensure prerequisites**  
  ```sh
  sudo apt install python3 python3-pip python3.12-venv
  ```
  Once installed, run `scripts/setup-gpt-oss.sh` from the repo root—it creates `scripts/.gpt-oss-env`, installs `transformers[serving]` + `torch`, and launches `transformers serve`.

- **Test the service**  
  ```sh
  curl -s http://localhost:8000/health
  transformers chat localhost:8000 --model-name-or-path openai/gpt-oss-20b
  ```
  The `/health` endpoint verifies the server is ready; the `transformers chat` command lets you interactively test the model.

- **Stop the server**  
  ```sh
  kill <pid>
  ```
  The setup script echos the PID (`SERVER_PID`), or use `ps -ef | grep transformers` to find it.

## 3. Notes for AIVM ↔ GPT-OSS

- Run AIVM RPC calls after the GPT-OSS server is accepting requests so AI-based contracts can call the local endpoint.
- Browser 404s on `/` or icon assets are normal—only the API endpoints are served, so rely on `/health` and RPC responses instead.
