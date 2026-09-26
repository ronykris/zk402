# zk402-groth16

Groth16 proof system implementation for zk402 using arkworks.

## Overview

This crate provides a concrete implementation of the `zk402_core::ProofSystem` trait using the Groth16 zero-knowledge proof system via the arkworks library ecosystem. It implements an EdDSA signature verification circuit that proves knowledge of a private key without revealing the signature itself.

## Features

- **Groth16 proving** - Efficient, constant-size proofs
- **EdDSA-in-circuit** - Signature verification inside the SNARK
- **BLS12-381 curve** - Well-supported, production-ready elliptic curve
- **Arkworks integration** - Built on the arkworks zkSNARK library suite

## Circuit Statement

The circuit proves knowledge of a secret key `sk` such that:

1. The public key derived from `sk` matches a known address
2. `sk` produced a valid EdDSA signature over the payment data (amount, recipient, nonce, etc.)

The signature itself is never revealed, only the proof of its validity.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
zk402-groth16 = "0.1.0"
zk402-core = "0.1.0"
```

Example:

```rust
use zk402_core::{ProofSystem, Witness, PublicInputs};
use zk402_groth16::Groth16ProofSystem;

// Load proving and verifying keys
let pk = Groth16ProofSystem::load_proving_key("path/to/pk.bin")?;
let vk = Groth16ProofSystem::load_verifying_key("path/to/vk.bin")?;

// Create witness data
let witness = Witness {
    secret_key: your_secret_key,
    nonce_scalar: nonce_bytes,
};

// Generate proof
let proof = Groth16ProofSystem::prove(&pk, &witness, &public_inputs)?;

// Verify proof
let valid = Groth16ProofSystem::verify(&vk, &proof, &public_inputs)?;
assert!(valid);
```

## Key Generation

Circuit parameters and proving/verifying keys must be generated via a trusted setup ceremony. This crate provides utilities for:

- Running a local trusted setup (for testing)
- Loading keys from binary files
- Serializing/deserializing keys

**Warning**: Use production trusted setup ceremonies for mainnet deployments, not local test setups.

## Performance

Typical benchmarks on modern hardware:

- Proving time: ~500ms - 2s (depending on witness complexity)
- Verification time: ~5-20ms
- Proof size: 192 bytes (constant)

## Security Considerations

- The circuit implements EdDSA (not ECDSA) for efficiency
- Public inputs include the sender address `from` (not private in v1)
- Nonce reuse is prevented by on-chain tracking
- Trusted setup parameters must come from a secure ceremony

## Related Crates

- `zk402-core` - Core types and traits
- `zk402-prover` - High-level prover interface
- `ark-groth16` - Underlying Groth16 implementation
- `ark-ed-on-bls12-381` - EdDSA curve

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
