# zk402-prover

Client-side proof generation for zk402 payments.

## Overview

This crate provides the client-side functionality for generating zero-knowledge payment proofs. It takes a witness (private key and payment data) along with payment requirements and produces a complete `PaymentPayload` ready to send to a facilitator.

## Features

- **High-level API** - Simple interface for proof generation
- **Witness management** - Safe handling of private key material
- **Payload assembly** - Automatic construction of wire-format payloads
- **Proof system agnostic** - Works with any `ProofSystem` implementation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
zk402-prover = "0.1.0"
zk402-core = "0.1.0"
zk402-groth16 = "0.1.0"
```

Example:

```rust
use zk402_prover::build_payment_payload;
use zk402_core::{PaymentRequirements, Witness};
use zk402_groth16::Groth16ProofSystem;

// Payment requirements from the payee
let requirements = PaymentRequirements {
    scheme: "zk-settle".to_string(),
    network: "eip155:8453".to_string(), // Base mainnet
    amount: "1000000".to_string(), // 1 USDC (6 decimals)
    asset: usdc_address,
    pay_to: recipient_address,
    max_timeout_seconds: 300,
    extra: ZkExtra {
        proof_system: "groth16".to_string(),
        circuit_id: "zk-settle-transfer-v1".to_string(),
        verifier_address,
    },
};

// Your private witness data
let witness = Witness {
    secret_key: your_private_key,
    nonce_scalar: random_nonce(),
};

// Public inputs
let public_inputs = PublicInputs {
    amount: requirements.amount.clone(),
    asset: requirements.asset,
    pay_to: requirements.pay_to,
    from: your_address,
    network: requirements.network.clone(),
    nonce: nonce_bytes,
    valid_after: current_timestamp(),
    valid_before: current_timestamp() + 300,
    public_key: derive_public_key(&witness.secret_key),
};

// Generate the payment payload
let payload = build_payment_payload::<Groth16ProofSystem>(
    &requirements,
    &witness,
    &public_inputs,
    &proving_key,
)?;

// Send payload to facilitator
// POST to /verify then /settle
```

## Security Best Practices

1. **Never reuse nonces** - Generate a fresh random nonce for each payment
2. **Secure key storage** - Keep private keys in secure enclaves/keystores
3. **Time windows** - Set reasonable `valid_after` and `valid_before` bounds
4. **Clear secrets** - Zero out witness data after proof generation

## Timing Considerations

- Proof generation is CPU-intensive (0.5-2 seconds typical)
- Consider showing a loading indicator in UI applications
- Can be run in a Web Worker (via WASM) to avoid blocking the main thread

## Related Crates

- `zk402-core` - Core types and traits
- `zk402-groth16` - Groth16 proof system backend
- `zk402-wasm` - WebAssembly bindings for browser clients
- `zk402-facilitator` - Server-side verification and settlement

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
