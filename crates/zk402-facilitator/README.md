# zk402-facilitator

Facilitator implementation for verifying and settling zk402 payments.

## Overview

This crate implements the server-side verification and settlement logic for the zk402 payment scheme. It provides the `ZkSettleFacilitator` which implements the `Scheme` trait, handling the five-step verification pipeline and on-chain settlement via Ethereum smart contracts.

## Features

- **Five-step verification** - Structural, trust, cryptographic, freshness, and simulation checks
- **On-chain settlement** - Direct integration with ZkSettleVerifier smart contract
- **Fail-closed design** - Any verification failure immediately rejects the payment
- **Configurable registry** - Pluggable verifying key registry for trust management
- **Ethereum integration** - Built on ethers-rs for robust chain interaction

## Verification Pipeline

The facilitator performs these checks in order:

1. **Structural match** - Public inputs match payment requirements
2. **Trust check** - Circuit ID is in the trusted verifying key registry
3. **Cryptographic verification** - Zero-knowledge proof is valid
4. **Freshness** - Payment is within valid time window
5. **Simulation** (optional) - Dry-run settlement to catch errors early

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
zk402-facilitator = "0.1.0"
zk402-core = "0.1.0"
zk402-groth16 = "0.1.0"
```

Example:

```rust
use zk402_facilitator::{ZkSettleFacilitator, StaticRegistry, SettlementConfig};
use zk402_core::{Scheme, PaymentRequirements, PaymentPayload};
use zk402_groth16::Groth16ProofSystem;

// Set up verifying key registry
let mut registry = StaticRegistry::new();
registry.register(
    "groth16",
    "zk-settle-transfer-v1",
    verifying_key,
);

// Configure settlement
let settlement_config = SettlementConfig::new(
    "https://mainnet.base.org".to_string(),
    verifier_contract_address,
).with_private_key(facilitator_private_key);

// Create facilitator
let facilitator = ZkSettleFacilitator::<Groth16ProofSystem>::new(
    registry,
    settlement_config,
);

// Verify a payment
let verify_result = facilitator.verify(&requirements, &payload).await?;
if !verify_result.is_valid {
    println!("Verification failed: {:?}", verify_result.invalid_reason);
    return;
}

// Settle on-chain
let settle_result = facilitator.settle(&requirements, &payload).await?;
if settle_result.success {
    println!("Settlement succeeded: {}", settle_result.tx_hash.unwrap());
} else {
    println!("Settlement failed: {:?}", settle_result.error);
}
```

## Settlement Configuration

The `SettlementConfig` requires:

- **RPC URL** - Ethereum JSON-RPC endpoint
- **Verifier address** - On-chain ZkSettleVerifier contract address
- **Private key** - (Optional) Facilitator signing key for transactions

```rust
let config = SettlementConfig::new(
    "https://mainnet.base.org".to_string(),
    [0x12, 0x34, ...], // 20-byte contract address
);
```

## Verifying Key Registry

The `StaticRegistry` loads trusted circuit verifying keys from configuration:

```rust
let mut registry = StaticRegistry::new();

// Register a trusted circuit
registry.register(
    "groth16",                      // proof system
    "zk-settle-transfer-v1",        // circuit ID
    groth16_verifying_key,
);

// Unknown circuits will be rejected
```

Future versions may support on-chain registries for decentralized trust.

## Error Handling

All verification failures return structured errors:

- `StructuralMismatch` - Public inputs don't match requirements
- `UnsupportedCircuit` - Circuit ID not in registry
- `CryptographicVerificationFailed` - Invalid proof
- `Expired` - Payment outside valid time window
- `ChainError` - On-chain transaction or RPC failure

## Security Invariants

The facilitator enforces these guarantees:

1. **I1: No redirection** - Funds always go to `pay_to` address
2. **I2: No replay** - Nonces are consumed on-chain
3. **I3: Atomicity** - Verify and settle are atomic on-chain
4. **I4: Trusted circuits** - Only registered verifying keys accepted
5. **I5: No custody** - Facilitator never holds user funds

## Related Crates

- `zk402-core` - Core types and traits
- `zk402-facilitator-http` - HTTP/REST API wrapper
- `zk402-groth16` - Groth16 proof verification
- `ethers` - Ethereum client library

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
