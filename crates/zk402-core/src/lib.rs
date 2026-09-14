//! Core types and traits for the zk402 payment scheme.
//!
//! This crate defines the wire format types, the `ProofSystem` trait boundary,
//! the `VerifyingKeyRegistry` trust model, and the `Scheme` trait that facilitators
//! implement. It has zero proving-library dependencies — all cryptographic backends
//! are swappable.

use serde::{Deserialize, Serialize};

pub type Address = [u8; 20];

/// Payment requirements from the payee, matching the x402 wire format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirements {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    #[serde(with = "hex_address")]
    pub asset: Address,
    #[serde(with = "hex_address")]
    pub pay_to: Address,
    pub max_timeout_seconds: u64,
    pub extra: ZkExtra,
}

/// ZK-specific fields within `PaymentRequirements.extra`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZkExtra {
    pub proof_system: String,
    pub circuit_id: String,
    #[serde(with = "hex_address")]
    pub verifier_address: Address,
}

/// Public inputs to the zero-knowledge proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicInputs {
    pub amount: String,
    #[serde(with = "hex_address")]
    pub asset: Address,
    #[serde(with = "hex_address")]
    pub pay_to: Address,
    #[serde(with = "hex_address")]
    pub from: Address,
    pub network: String,
    #[serde(with = "hex_nonce")]
    pub nonce: [u8; 32],
    pub valid_after: u64,
    pub valid_before: u64,
    /// Compressed public key (EdDSA) for signature verification
    #[serde(with = "hex_bytes")]
    pub public_key: Vec<u8>,
}

/// Complete payment payload sent by the payer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentPayload {
    pub x402_version: u8,
    pub accepted: PaymentRequirements,
    #[serde(with = "hex_bytes")]
    pub proof: Vec<u8>,
    pub public_inputs: PublicInputs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_system_extra: Option<serde_json::Value>,
}

/// Response from the verify endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub is_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_reason: Option<String>,
}

/// Response from the settle endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Abstract proof system backend — the primary swap point in the architecture.
///
/// `zk402-groth16` implements this; future backends (Halo2, etc.) implement the same
/// trait without changes to consumers.
pub trait ProofSystem {
    type ProvingKey;
    type VerifyingKey;

    fn prove(
        pk: &Self::ProvingKey,
        witness: &Witness,
        public_inputs: &PublicInputs,
    ) -> Result<Vec<u8>, ProofSystemError>;

    fn verify(
        vk: &Self::VerifyingKey,
        proof: &[u8],
        public_inputs: &PublicInputs,
    ) -> Result<bool, ProofSystemError>;
}

/// Witness data for proof generation (private inputs).
#[derive(Clone, Debug)]
pub struct Witness {
    pub secret_key: Vec<u8>,
    pub nonce_scalar: Vec<u8>,
}

#[derive(Debug)]
pub enum ProofSystemError {
    InvalidParameters,
    ProvingFailed(String),
    VerificationFailed(String),
    Encoding(String),
}

impl std::fmt::Display for ProofSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParameters => write!(f, "invalid proof system parameters"),
            Self::ProvingFailed(msg) => write!(f, "proving failed: {msg}"),
            Self::VerificationFailed(msg) => write!(f, "verification failed: {msg}"),
            Self::Encoding(msg) => write!(f, "encoding error: {msg}"),
        }
    }
}

impl std::error::Error for ProofSystemError {}

/// Verifying key registry — the trust model swap point.
///
/// v1 is `StaticRegistry` (config file); future versions might use on-chain lookups.
pub trait VerifyingKeyRegistry {
    type VerifyingKeyHandle;

    fn lookup(
        &self,
        proof_system: &str,
        circuit_id: &str,
    ) -> Option<Self::VerifyingKeyHandle>;
}

/// Scheme trait matching the shape existing facilitator implementations use.
#[async_trait::async_trait]
pub trait Scheme {
    fn scheme_name(&self) -> &str;

    async fn verify(
        &self,
        req: &PaymentRequirements,
        payload: &PaymentPayload,
    ) -> Result<VerifyResponse, SchemeError>;

    async fn settle(
        &self,
        req: &PaymentRequirements,
        payload: &PaymentPayload,
    ) -> Result<SettleResponse, SchemeError>;
}

#[derive(Debug)]
pub enum SchemeError {
    StructuralMismatch(String),
    UnsupportedCircuit,
    CryptographicVerificationFailed,
    Expired,
    ChainError(String),
    Internal(String),
}

impl std::fmt::Display for SchemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StructuralMismatch(msg) => write!(f, "structural mismatch: {msg}"),
            Self::UnsupportedCircuit => write!(f, "unsupported circuit"),
            Self::CryptographicVerificationFailed => write!(f, "proof verification failed"),
            Self::Expired => write!(f, "payment window expired or not yet valid"),
            Self::ChainError(msg) => write!(f, "chain error: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for SchemeError {}

// Hex serialization helpers for addresses and nonces
mod hex_address {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 20], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = format!("0x{}", hex::encode(bytes));
        serializer.serialize_str(&hex_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 20], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.strip_prefix("0x").unwrap_or(&s);
        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
        bytes.try_into().map_err(|_| {
            serde::de::Error::custom("expected 20 bytes for address")
        })
    }
}

mod hex_nonce {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = format!("0x{}", hex::encode(bytes));
        serializer.serialize_str(&hex_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.strip_prefix("0x").unwrap_or(&s);
        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
        bytes.try_into().map_err(|_| {
            serde::de::Error::custom("expected 32 bytes for nonce")
        })
    }
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = format!("0x{}", hex::encode(bytes));
        serializer.serialize_str(&hex_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.strip_prefix("0x").unwrap_or(&s);
        hex::decode(s).map_err(serde::de::Error::custom)
    }
}
