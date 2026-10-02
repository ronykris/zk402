# zk402

A standalone Rust implementation of a zero-knowledge Agentic AI payment settlement scheme, enabling privacy-preserving payment authorization via SNARKs.

## Overview

zk402 is a complete implementation of a zero-knowledge payment protocol where payers can authorize payments by generating cryptographic proofs instead of exposing signatures. This provides:

- **Privacy** - Payment authorization without revealing signatures
- **Flexibility** - Extensible circuit design for custom payment predicates
- **Interoperability** - Compatible with ERC-20 tokens on EVM chains
- **Non-custodial** - Facilitators never hold user funds

## Architecture

The project is structured as a Rust workspace with clear separation between:

- **Cryptographic core** (`zk402-groth16`) - Proof system implementation
- **Protocol logic** (`zk402-core`) - Wire format types and traits
- **Client-side** (`zk402-prover`, `zk402-wasm`) - Proof generation
- **Server-side** (`zk402-facilitator`, `zk402-facilitator-http`) - Verification and settlement
- **Smart contracts** (`contracts/`) - On-chain verifier and settlement logic

```
zk402/
├── crates/
│   ├── zk402-core/                # Core types, traits, wire format
│   ├── zk402-groth16/             # Groth16 proof system (arkworks)
│   ├── zk402-prover/              # Client-side proof generation
│   ├── zk402-facilitator/         # Verification & settlement logic
│   ├── zk402-facilitator-http/    # HTTP API service
│   └── zk402-wasm/                # WebAssembly bindings
└── contracts/                     # Solidity verifier + settlement
```

## Quick Start

### Prerequisites

- Rust 1.80+ ([install](https://rustup.rs/))
- Node.js 18+ (for WASM bindings)
- Foundry (for smart contracts: [install](https://book.getfoundry.sh/getting-started/installation))

### Build Everything

```bash
# Clone the repository
git clone https://github.com/zk402/zk402.git
cd zk402

# Build all Rust crates
cargo build --release

# Build smart contracts
cd contracts && forge build
```

### Run the Facilitator Service

```bash
# Set environment variables
export RPC_URL=https://mainnet.base.org
export VERIFIER_ADDRESS=0x1234567890123456789012345678901234567890
export FACILITATOR_PRIVATE_KEY=0xabcdef...

# Run the HTTP service
cargo run --release --bin zk402-facilitator-http
```

The service will start on `http://localhost:3000` with endpoints:
- `POST /verify` - Verify a zero-knowledge payment proof
- `POST /settle` - Settle a verified payment on-chain
- `GET /health` - Health check

### Generate a Proof (Browser)

```typescript
import init, { build_payload } from 'zk402-wasm';

await init();

const payload = build_payload(
  JSON.stringify(requirements),
  JSON.stringify(witness)
);

// Send to facilitator
const response = await fetch('http://localhost:3000/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ requirements, payload: JSON.parse(payload) }),
});
```

## Crates

### Core Types (`zk402-core`)

Foundation types and traits with zero cryptographic dependencies. Defines the wire format, `ProofSystem` trait, `VerifyingKeyRegistry` trait, and `Scheme` trait.

- [Documentation](./crates/zk402-core/README.md)
- Published: `cargo add zk402-core`

### Groth16 Backend (`zk402-groth16`)

Concrete proof system implementation using arkworks. Implements EdDSA signature verification in a SNARK circuit.

- [Documentation](./crates/zk402-groth16/README.md)
- Published: `cargo add zk402-groth16`

### Client Prover (`zk402-prover`)

High-level API for client-side proof generation. Takes witness data and payment requirements, produces a complete `PaymentPayload`.

- [Documentation](./crates/zk402-prover/README.md)
- Published: `cargo add zk402-prover`

### Facilitator (`zk402-facilitator`)

Server-side verification and settlement logic. Implements the five-step verification pipeline and on-chain settlement via ethers-rs.

- [Documentation](./crates/zk402-facilitator/README.md)
- Published: `cargo add zk402-facilitator`

### HTTP Service (`zk402-facilitator-http`)

Production HTTP/REST API built with Axum. Exposes verify and settle endpoints matching the x402 facilitator contract.

- [Documentation](./crates/zk402-facilitator-http/README.md)
- Runnable: `cargo run --bin zk402-facilitator-http`

### WASM Bindings (`zk402-wasm`)

WebAssembly bindings for browser and Node.js clients. Enables proof generation in JavaScript/TypeScript environments.

- [Documentation](./crates/zk402-wasm/README.md)
- Published: `npm install zk402-wasm`

## Smart Contracts

Solidity contracts for on-chain verification and settlement:

- `ZkSettleVerifier.sol` - Main settlement contract with proof verification
- `IProofVerifier.sol` - Interface for pluggable verifier backends
- `IERC20.sol` - ERC-20 token interface

### Deploy Contracts

```bash
cd contracts

# Deploy to local anvil
forge script script/Deploy.s.sol:Deploy --rpc-url http://localhost:8545 --broadcast

# Deploy to Base mainnet
forge script script/Deploy.s.sol:Deploy --rpc-url $RPC_URL --broadcast --verify
```

## How It Works

1. **Payee** publishes payment requirements (amount, asset, recipient, etc.)
2. **Payer** generates a zero-knowledge proof of payment authorization
3. **Facilitator** verifies the proof cryptographically
4. **Settlement** contract atomically verifies + transfers funds on-chain

### Security Invariants

- **I1: No redirection** - Funds always go to the specified `payTo` address
- **I2: No replay** - Nonces are consumed on-chain to prevent reuse
- **I3: Atomicity** - Verify and settle happen in one transaction
- **I4: Trusted circuits** - Only registered verifying keys are accepted
- **I5: No custody** - Facilitator never holds user funds mid-flow

## Development

### Running Tests

```bash
# Run all Rust tests
cargo test --workspace

# Run Solidity tests
cd contracts && forge test
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --all-targets --all-features

# Check smart contracts
cd contracts && forge fmt && forge check
```

## Use Cases

- **Confidential payments** - Authorize transfers without exposing signatures
- **Batched authorizations** - Single proof for multiple payment checks
- **Custom predicates** - Extend circuits for balance checks, allowlists, etc.
- **Cross-chain settlement** - ZK proofs as portable payment authorizations

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass (`cargo test --workspace`)
5. Submit a pull request

## Security

This is experimental software. Use at your own risk.

For security issues, please contact: security@zk402.org (or file a private security advisory)


## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Resources

- [Architecture Documentation](./zk402-architecture.md)
- [Wire Format Specification](./docs/wire-format.md) (TBD)
- [Circuit Statement](./docs/circuit.md) (TBD)
- [API Reference](https://docs.rs/zk402-core)

## Acknowledgments

Built with:

- [arkworks](https://github.com/arkworks-rs) - Zero-knowledge proof libraries
- [ethers-rs](https://github.com/gakonst/ethers-rs) - Ethereum client
- [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) - Rust/WASM interop
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Foundry](https://github.com/foundry-rs/foundry) - Smart contract toolkit
