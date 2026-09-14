//! zk402 facilitator implementation
//!
//! Implements the five-step verification pipeline and settlement logic
//! for the zk402 payment scheme.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use zk402_core::*;

/// Static verifying key registry loaded from configuration.
pub struct StaticRegistry<VK> {
    keys: HashMap<(String, String), VK>,
}

impl<VK: Clone> StaticRegistry<VK> {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    pub fn register(&mut self, proof_system: String, circuit_id: String, vk: VK) {
        self.keys.insert((proof_system, circuit_id), vk);
    }
}

impl<VK: Clone> Default for StaticRegistry<VK> {
    fn default() -> Self {
        Self::new()
    }
}

impl<VK: Clone> VerifyingKeyRegistry for StaticRegistry<VK> {
    type VerifyingKeyHandle = VK;

    fn lookup(
        &self,
        proof_system: &str,
        circuit_id: &str,
    ) -> Option<Self::VerifyingKeyHandle> {
        self.keys
            .get(&(proof_system.to_string(), circuit_id.to_string()))
            .cloned()
    }
}

/// zk402 facilitator implementing the Scheme trait.
pub struct ZkSettleFacilitator<PS: ProofSystem, VKR: VerifyingKeyRegistry> {
    registry: VKR,
    _phantom: std::marker::PhantomData<PS>,
}

impl<PS: ProofSystem, VKR: VerifyingKeyRegistry<VerifyingKeyHandle = PS::VerifyingKey>>
    ZkSettleFacilitator<PS, VKR>
{
    pub fn new(registry: VKR) -> Self {
        Self {
            registry,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Step 1: Structural match validation
    fn validate_structural_match(
        req: &PaymentRequirements,
        payload: &PaymentPayload,
    ) -> Result<(), SchemeError> {
        let pi = &payload.public_inputs;

        if pi.amount != req.amount {
            return Err(SchemeError::StructuralMismatch(format!(
                "amount mismatch: expected {}, got {}",
                req.amount, pi.amount
            )));
        }

        if pi.asset != req.asset {
            return Err(SchemeError::StructuralMismatch(format!(
                "asset mismatch: expected 0x{}, got 0x{}",
                hex::encode(req.asset),
                hex::encode(pi.asset)
            )));
        }

        if pi.pay_to != req.pay_to {
            return Err(SchemeError::StructuralMismatch(format!(
                "payTo mismatch: expected 0x{}, got 0x{}",
                hex::encode(req.pay_to),
                hex::encode(pi.pay_to)
            )));
        }

        if pi.network != req.network {
            return Err(SchemeError::StructuralMismatch(format!(
                "network mismatch: expected {}, got {}",
                req.network, pi.network
            )));
        }

        Ok(())
    }

    /// Step 2: Trust check - lookup verifying key
    fn trust_check(&self, extra: &ZkExtra) -> Result<PS::VerifyingKey, SchemeError> {
        self.registry
            .lookup(&extra.proof_system, &extra.circuit_id)
            .ok_or(SchemeError::UnsupportedCircuit)
    }

    /// Step 3: Cryptographic verification
    fn cryptographic_verification(
        vk: &PS::VerifyingKey,
        payload: &PaymentPayload,
    ) -> Result<(), SchemeError> {
        PS::verify(vk, &payload.proof, &payload.public_inputs)
            .map_err(|_| SchemeError::Internal("proof verification error".to_string()))?
            .then_some(())
            .ok_or(SchemeError::CryptographicVerificationFailed)
    }

    /// Step 4: Freshness check
    fn freshness_check(public_inputs: &PublicInputs) -> Result<(), SchemeError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SchemeError::Internal(format!("system time error: {}", e)))?
            .as_secs();

        if now < public_inputs.valid_after {
            return Err(SchemeError::Expired);
        }

        if now >= public_inputs.valid_before {
            return Err(SchemeError::Expired);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<PS: ProofSystem + Send + Sync, VKR: VerifyingKeyRegistry<VerifyingKeyHandle = PS::VerifyingKey> + Send + Sync>
    Scheme for ZkSettleFacilitator<PS, VKR>
{
    fn scheme_name(&self) -> &str {
        "zk-settle"
    }

    async fn verify(
        &self,
        req: &PaymentRequirements,
        payload: &PaymentPayload,
    ) -> Result<VerifyResponse, SchemeError> {
        // Step 1: Structural match
        Self::validate_structural_match(req, payload)?;

        // Step 2: Trust check
        let vk = self.trust_check(&req.extra)?;

        // Step 3: Cryptographic verification
        Self::cryptographic_verification(&vk, payload)?;

        // Step 4: Freshness
        Self::freshness_check(&payload.public_inputs)?;

        // Step 5: Simulation (optional for v1) - not implemented yet

        Ok(VerifyResponse {
            is_valid: true,
            invalid_reason: None,
        })
    }

    async fn settle(
        &self,
        _req: &PaymentRequirements,
        _payload: &PaymentPayload,
    ) -> Result<SettleResponse, SchemeError> {
        // Stubbed for Phase 3 - will be implemented in Phase 3.5
        Ok(SettleResponse {
            success: true,
            tx_hash: Some("0x0000000000000000000000000000000000000000000000000000000000000000".to_string()),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProofSystem;

    impl ProofSystem for MockProofSystem {
        type ProvingKey = Vec<u8>;
        type VerifyingKey = Vec<u8>;

        fn prove(
            _pk: &Self::ProvingKey,
            _witness: &Witness,
            _public_inputs: &PublicInputs,
        ) -> Result<Vec<u8>, ProofSystemError> {
            Ok(vec![0x42])
        }

        fn verify(
            _vk: &Self::VerifyingKey,
            proof: &[u8],
            _public_inputs: &PublicInputs,
        ) -> Result<bool, ProofSystemError> {
            // Valid if proof is [0x42], invalid otherwise
            Ok(proof == &[0x42])
        }
    }

    fn test_requirements() -> PaymentRequirements {
        PaymentRequirements {
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
        }
    }

    fn test_payload(proof: Vec<u8>) -> PaymentPayload {
        let req = test_requirements();
        PaymentPayload {
            x402_version: 1,
            accepted: req.clone(),
            proof,
            public_inputs: PublicInputs {
                amount: req.amount.clone(),
                asset: req.asset,
                pay_to: req.pay_to,
                from: [4u8; 20],
                network: req.network.clone(),
                nonce: [5u8; 32],
                valid_after: current_timestamp() - 60,
                valid_before: current_timestamp() + 3600,
                public_key: vec![6u8; 32],
            },
            proof_system_extra: None,
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[tokio::test]
    async fn valid_proof_passes_verification() {
        let mut registry = StaticRegistry::new();
        registry.register(
            "groth16".to_string(),
            "zk-settle-transfer-v1".to_string(),
            vec![0u8],
        );

        let facilitator = ZkSettleFacilitator::<MockProofSystem, _>::new(registry);
        let req = test_requirements();
        let payload = test_payload(vec![0x42]);

        let result = facilitator.verify(&req, &payload).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_valid);
    }

    #[tokio::test]
    async fn tampered_amount_fails_structural_match() {
        let mut registry = StaticRegistry::new();
        registry.register(
            "groth16".to_string(),
            "zk-settle-transfer-v1".to_string(),
            vec![0u8],
        );

        let facilitator = ZkSettleFacilitator::<MockProofSystem, _>::new(registry);
        let req = test_requirements();
        let mut payload = test_payload(vec![0x42]);
        payload.public_inputs.amount = "101".to_string();

        let result = facilitator.verify(&req, &payload).await;
        assert!(matches!(result, Err(SchemeError::StructuralMismatch(_))));
    }

    #[tokio::test]
    async fn unknown_circuit_id_fails_trust_check() {
        let registry = StaticRegistry::<Vec<u8>>::new();
        let facilitator = ZkSettleFacilitator::<MockProofSystem, _>::new(registry);
        let req = test_requirements();
        let payload = test_payload(vec![0x42]);

        let result = facilitator.verify(&req, &payload).await;
        assert!(matches!(result, Err(SchemeError::UnsupportedCircuit)));
    }

    #[tokio::test]
    async fn invalid_proof_fails_crypto_verification() {
        let mut registry = StaticRegistry::new();
        registry.register(
            "groth16".to_string(),
            "zk-settle-transfer-v1".to_string(),
            vec![0u8],
        );

        let facilitator = ZkSettleFacilitator::<MockProofSystem, _>::new(registry);
        let req = test_requirements();
        let payload = test_payload(vec![0x99]); // Invalid proof

        let result = facilitator.verify(&req, &payload).await;
        assert!(matches!(
            result,
            Err(SchemeError::CryptographicVerificationFailed)
        ));
    }

    #[tokio::test]
    async fn expired_window_fails_freshness() {
        let mut registry = StaticRegistry::new();
        registry.register(
            "groth16".to_string(),
            "zk-settle-transfer-v1".to_string(),
            vec![0u8],
        );

        let facilitator = ZkSettleFacilitator::<MockProofSystem, _>::new(registry);
        let req = test_requirements();
        let mut payload = test_payload(vec![0x42]);
        payload.public_inputs.valid_before = current_timestamp() - 10; // Already expired

        let result = facilitator.verify(&req, &payload).await;
        assert!(matches!(result, Err(SchemeError::Expired)));
    }
}
