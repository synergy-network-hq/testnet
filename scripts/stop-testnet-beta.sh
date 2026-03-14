#!/bin/bash

# Stop the Synergy Testnet Beta Node

echo "🛑 Stopping Synergy Testnet Beta..."

cd "$(dirname "$0")/.."

if [ -f data/synergy-testbeta.pid ]; then
    NODE_PID=$(cat data/synergy-testbeta.pid)

    if ps -p "$NODE_PID" > /dev/null; then
        kill "$NODE_PID"
        echo "✅ Node process $NODE_PID terminated."
    else
        echo "⚠️ PID $NODE_PID not running. Skipping kill."
    fi

    rm -f data/synergy-testbeta.pid
else
    echo "⚠️ No PID file found. Attempting to kill any matching synergy-testbeta processes..."
    pkill -f synergy-testbeta || echo "No processes matched."
fi

echo "🧹 Synergy Testnet Beta shutdown complete."
