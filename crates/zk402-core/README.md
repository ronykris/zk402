# zk402-core

Core types and traits for the zk402 zero-knowledge payment scheme.

## Overview

This crate defines the foundational types, traits, and wire format for the zk402 payment protocol. It provides the abstraction boundaries that allow different proof systems, verifying key registries, and scheme implementations to be swapped without affecting other components.

## Key Features

- **Zero dependencies on cryptographic libraries** - All proving backends are swappable
- **Wire format types** - Matching the x402 specification
- **`ProofSystem` trait** - Abstract interface for proof generation and verification
- **`VerifyingKeyRegistry` trait** - Pluggable trust model for circuit verification keys
- **`Scheme` trait** - Standard interface for facilitator implementations

## Types

### Wire Format Types

- `PaymentRequirements` - Payment terms from the payee
- `PaymentPayload` - Complete payment data from the payer
- `PublicInputs` - Public inputs to the zero-knowledge proof
- `ZkExtra` - ZK-specific metadata

### Response Types

- `VerifyResponse` - Result of proof verification
- `SettleResponse` - Result of on-chain settlement

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
zk402-core = "0.1.0"
```

Example of using the core types:

```rust
use zk402_core::{PaymentRequirements, PublicInputs, ProofSystem};

// Define your proof system implementation
struct MyProofSystem;

impl ProofSystem for MyProofSystem {
    type ProvingKey = MyProvingKey;
    type VerifyingKey = MyVerifyingKey;

    fn prove(
        pk: &Self::ProvingKey,
        witness: &Witness,
        public_inputs: &PublicInputs,
    ) -> Result<Vec<u8>, ProofSystemError> {
        // Your proving logic
    }

    fn verify(
        vk: &Self::VerifyingKey,
        proof: &[u8],
        public_inputs: &PublicInputs,
    ) -> Result<bool, ProofSystemError> {
        // Your verification logic
    }
}
```

## Architecture

The `zk402-core` crate is intentionally minimal and has no cryptographic dependencies. This design allows:

1. **Proof system flexibility** - Swap between Groth16, Halo2, or other systems
2. **Trust model flexibility** - Use static configs, on-chain registries, or custom solutions
3. **Implementation flexibility** - Build facilitators, provers, or custom clients

## Related Crates

- `zk402-groth16` - Groth16 proof system implementation
- `zk402-prover` - Client-side proof generation
- `zk402-facilitator` - Facilitator verify/settle implementation
- `zk402-wasm` - WebAssembly bindings for browser clients

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
