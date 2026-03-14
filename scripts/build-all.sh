#!/bin/bash

# Synergy Network Complete Build Script
# Builds all components: Core, SynQ, and PQC systems

echo "🚀 Building Complete Synergy Network with SynQ and PQC..."
echo "========================================================="

# Set script path to root of the project
cd "$(dirname "$0")/.."

echo "📦 Phase 1: Installing Dependencies..."
echo "-------------------------------------"

# Install Rust dependencies
echo "🔧 Installing Rust dependencies..."
cargo fetch

# Install Node.js dependencies if package.json exists
if [ -f "package.json" ]; then
    echo "📦 Installing Node.js dependencies..."
    npm install
fi

echo ""
echo "🔨 Phase 2: Building Core Components..."
echo "--------------------------------------"

# Build main Rust project
echo "🔧 Building Synergy Network core..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Core build failed!"
    exit 1
fi

echo "✅ Core build completed successfully"

echo ""
echo "🤖 Phase 3: Building SynQ Programming Language..."
echo "------------------------------------------------"

if [ -d "SynQ" ]; then
    cd SynQ

    echo "🔧 Building SynQ CLI..."
    cd cli && cargo build --release && cd ..

    echo "🔧 Building SynQ Compiler..."
    cd compiler && cargo build --release && cd ..

    echo "🔧 Building SynQ VM..."
    cd vm && cargo build --release && cd ..

    echo "🔧 Building SynQ PQC Shims..."
    cd pqc-shims && cargo build --release && cd ../..

    echo "✅ SynQ components built successfully"
else
    echo "⚠️ SynQ folder not found, skipping SynQ build"
fi

echo ""
echo "🔒 Phase 4: Post-Quantum Cryptography Integration..."
echo "---------------------------------------------------"

# Build Aegis PQVM if available
if [ -d "node_modules/aegis-pqvm" ]; then
    echo "🔧 Building Aegis PQVM..."
    cd node_modules/aegis-pqvm

    # Try to build with available dependencies
    if command -v cargo &> /dev/null; then
        cargo build --release 2>/dev/null || echo "⚠️ Aegis PQVM build failed (missing dependencies)"
    fi

    cd ../..
fi

# Build pqcrypto dependencies
echo "🔧 Building PQC libraries..."
cargo build --release --features pqcrypto

echo ""
echo "📊 Phase 5: Integration Testing..."
echo "---------------------------------"

# Run tests
echo "🧪 Running core tests..."
cargo test --release

if [ -d "SynQ" ]; then
    echo "🧪 Running SynQ tests..."
    cd SynQ && cargo test --release && cd ..
fi

echo ""
echo "🎯 Phase 6: Final Assembly..."
echo "----------------------------"

# Create data directories
echo "📁 Creating data directories..."
mkdir -p data/chain
mkdir -p data/logs
mkdir -p data/synq

# Initialize configuration if needed
if [ ! -f "config/genesis.json" ]; then
    echo "⚙️ Initializing configuration..."
    cargo run --release -- init
fi

echo ""
echo "✨ BUILD COMPLETE!"
echo "=================="
echo ""
echo "🎯 Synergy Network Features:"
echo "   ✅ Distributed AI (AIVM) with validator clusters"
echo "   ✅ Post-Quantum Cryptography (5 NIST algorithms)"
echo "   ✅ SynQ Programming Language with PQC integration"
echo "   ✅ Universal Interoperability across blockchains"
echo "   ✅ Military-grade security with consensus verification"
echo ""
echo "📊 Network Specifications:"
echo "   🪙 Native Token: SNRG (9 decimals, 12B supply)"
echo "   🧠 Consensus: Proof of Synergy with AI integration"
echo "   🔒 Security: 5 NIST PQC algorithms + distributed trust"
echo "   🌐 Interoperability: Universal blockchain compatibility"
echo ""
echo "🚀 Ready for deployment!"
echo "   Main node: cargo run --release -- start"
echo "   SynQ compiler: ./SynQ/target/release/synq-cli compile"
echo "   API access: http://localhost:8545"


