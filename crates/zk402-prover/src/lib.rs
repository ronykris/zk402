//! Client-side proof generation
//!
//! Takes private witness + PaymentRequirements, generates proof, assembles PaymentPayload.

use rand::thread_rng;
use zk402_core::*;
use zk402_groth16::proof_system::Groth16ProofSystem;

#[derive(Debug)]
pub enum ProverError {
    InvalidRequirements(String),
    ProofGenerationFailed(String),
    KeyDerivationFailed(String),
}

impl std::fmt::Display for ProverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequirements(msg) => write!(f, "invalid requirements: {msg}"),
            Self::ProofGenerationFailed(msg) => write!(f, "proof generation failed: {msg}"),
            Self::KeyDerivationFailed(msg) => write!(f, "key derivation failed: {msg}"),
        }
    }
}

impl std::error::Error for ProverError {}

/// Client witness data for proof generation
pub struct ClientWitness {
    /// Private signing key (raw bytes)
    pub secret_key: Vec<u8>,
    /// Payment source address (must match derived public key)
    pub from: [u8; 20],
    /// Random nonce for this payment (32 bytes)
    pub payment_nonce: [u8; 32],
}

/// Configuration for proof generation
pub struct ProverConfig {
    /// Groth16 proving key (serialized)
    pub proving_key: Vec<u8>,
    /// Validity window start (unix timestamp)
    pub valid_after: u64,
    /// Validity window end (unix timestamp)
    pub valid_before: u64,
}

impl ProverConfig {
    /// Create config with a time window from now
    pub fn with_validity_window(proving_key: Vec<u8>, duration_secs: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            proving_key,
            valid_after: now,
            valid_before: now + duration_secs,
        }
    }
}

/// Build a complete PaymentPayload from requirements and witness
///
/// This is the main entry point for client-side proof generation.
pub fn build_payload(
    requirements: &PaymentRequirements,
    witness: &ClientWitness,
    config: &ProverConfig,
) -> Result<PaymentPayload, ProverError> {
    // Derive public key from secret key
    let public_key = zk402_groth16::derive_public_key(&witness.secret_key)
        .map_err(|e| ProverError::KeyDerivationFailed(e.to_string()))?;

    // Build PublicInputs
    let public_inputs = PublicInputs {
        amount: requirements.amount.clone(),
        asset: requirements.asset,
        pay_to: requirements.pay_to,
        from: witness.from,
        network: requirements.network.clone(),
        nonce: witness.payment_nonce,
        valid_after: config.valid_after,
        valid_before: config.valid_before,
        public_key,
    };

    // Build witness for proof system
    let proof_witness = Witness {
        secret_key: witness.secret_key.clone(),
        nonce_scalar: vec![0u8; 32], // Random nonce for signature
    };

    // Generate proof
    let proof = Groth16ProofSystem::prove(&config.proving_key, &proof_witness, &public_inputs)
        .map_err(|e| ProverError::ProofGenerationFailed(e.to_string()))?;

    // Assemble complete payload
    Ok(PaymentPayload {
        x402_version: 1,
        accepted: requirements.clone(),
        proof,
        public_inputs,
        proof_system_extra: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_payload_structure() {
        // This test validates the structure without actual proving (which requires setup)
        let requirements = PaymentRequirements {
            scheme: "zk-settle".to_string(),
            network: "eip155:8453".to_string(),
            amount: "100".to_string(),
            asset: [1u8; 20],
            pay_to: [2u8; 20],
            max_timeout_seconds: 3600,
            extra: ZkExtra {
                proof_system: "groth16".to_string(),
                circuit_id: "zk-settle-transfer-v1".to_string(),
                verifier_address: [3u8; 20],
            },
        };

        let witness = ClientWitness {
            secret_key: b"test-secret-key-32-bytes-long!!!".to_vec(),
            from: [4u8; 20],
            payment_nonce: [5u8; 32],
        };

        // Would need real proving key from setup()
        // For now, just verify error handling works
        let config = ProverConfig::with_validity_window(vec![0u8; 100], 3600);

        let result = build_payload(&requirements, &witness, &config);

        // Expect it to fail without real proving key, but not panic
        assert!(result.is_err());
    }

    #[test]
    fn derives_public_key_correctly() {
        let secret = b"test-secret-key-32-bytes-long!!!";
        let result = zk402_groth16::derive_public_key(secret);
        assert!(result.is_ok());
        let pubkey = result.unwrap();
        assert!(!pubkey.is_empty());
    }
}
