# SynQ

SynQ is a domain-specific language (DSL) designed for writing **quantum-resistant smart contracts** using NIST-standardized post-quantum cryptographic (PQC) algorithms such as **Dilithium**, **Falcon**, and **Kyber**. It enables the development of secure decentralized applications (dApps) that remain resilient in the face of future quantum computing threats.

---

## 🔒 Post-Quantum Cryptography Support
SynQ natively supports:

| Algorithm | Purpose | Type(s) |
|----------|---------|---------|
| Dilithium | Digital Signatures | `DilithiumKeyPair<L>`, `DilithiumSignature<L>` |
| Falcon | Compact Signatures | `FalconKeyPair<N>`, `FalconSignature<N>` |
| Kyber | Key Encapsulation | `KyberKeyPair<L>`, `KyberCiphertext<L>` |

> Security levels are encoded in type parameters (e.g. `<3>` for Dilithium-3, `<768>` for Kyber-768).

---

## 📦 Features

### ✅ First-Class Cryptographic Types
- Strong type enforcement prevents security mismatches
- Parameterized types by security level
- Composite authentication via `PQAuth`

### ⚙️ Explicit Gas Accounting
- `@gas_cost(base, per_op)` decorator for every PQC operation
- Gas costs based on compute, input size, and key strength
- `@optimize_gas` and batch ops supported

### 🔐 Signature Enforcement
- `require_pqc { ... }` block enforces PQC verifications
- `authenticated_pqc` modifier for secure execution paths

### 🧠 VM Integration
- Uses precompiled contracts for PQC ops
- Tracks PQ-Gas separately from standard gas
- Optional support for hardware acceleration (HSM, TPM, etc.)

---

## 🧰 Core Syntax

### 🔧 Types
```quantumscript
type DilithiumKeyPair<3>
type FalconSignature<1024>
type KyberCiphertext<768>
```

### 🔑 Composite Authentication
```quantumscript
type PQAuth = {
    dilithium_key: DilithiumKeyPair<3>,
    falcon_key: FalconKeyPair<1024>,
    backup_key: DilithiumKeyPair<2>
}
```

### 🧪 Signature Verification
```quantumscript
require_pqc {
    verify_dilithium<3>(admin_key, msg, sig);
} or revert("Invalid sig");
```

### 💸 Gas Budgeting
```quantumscript
@gas_cost(base: 75000, dilithium_verify: 35000)
function submit_proposal(...) { ... }
```

---

## 🏛 Example: PQC-Verified DAO
SynQ includes a full-featured DAO contract example with:
- Admin control via Dilithium-3
- Voting via encrypted Falcon + Kyber
- Proposal submission, encrypted vote casting, batched tally
- Governance key rotation with `verify_dilithium`

> See: `Quantum Dao Contract`

---

## ⚙️ Development Tools

### 🛠 CLI Compiler
```bash
$ qsc compile QuantumDAO.qs --target kyber-768
$ qsc deploy --contract QuantumDAO --gas-overhead 15000
$ qsc estimate --function cast_vote --args ...
```

### 🧪 Simulation Tools
- `qsc simulate` — test gas use and verify PQ-Gas capping
- `qsc trace` — debug `require_pqc` branches

---

## 🔐 Security Model
- All critical contract paths gated by post-quantum signatures
- No use of classical (ECDSA, Ed25519) keys
- Addresses and contracts use Bech32m encoding
- Gas overuse trapped via VM-level `PQGasTracker`
- Signature domain prefixing (`"VOTE:"`, `"PROPOSAL:"`) is mandatory

---

## 🔮 Future Features
- zk-Dilithium and zk-KEM proof verification
- Optional PQC signature aggregations
- Module import system (`use pqc::falcon`)
- Interoperability with classical and quantum-native chains
- Proof-based cold wallet recovery

---

## 📚 Files
| File | Description |
|------|-------------|
| `Quantumscript Dsl` | Core language syntax and types |
| `Quantum Script Gas Model` | Full resource and cost economics |
| `Quantum Dao Contract` | Reference DAO with full PQC controls |
| `Quantum Script Vm Spec` | Precompile/VM runtime architecture |

---

## 🤝 Contributing
To contribute:
1. Fork this repo
2. Clone and run `qsc` locally
3. Modify one of the source documents
4. Submit a PR with `[SynQ]` prefix

### 📜 Coding Guidelines
- All PQC types must include `<level>` param
- Signature and encryption messages must be ABI-encoded and prefixed
- All public functions must declare `@gas_cost`

---

## 👨‍🚀 Maintainers
SynQ is maintained by the Synergy Network Core R&D team. For protocol-level discussions, visit the [Synergy DevNet Forum](https://forum.synergynet.dev).

---

## 🧠 License
SynQ is released under the MIT License.

---

End of README.md
