//! WASM bindings for zk402 client-side proof generation
//!
//! Exposes a single function: `build_payload(requirements_json, witness_json) -> payload_json`

use wasm_bindgen::prelude::*;
use zk402_prover::{build_payload as prover_build_payload, ClientWitness, ProverConfig};

/// Set up panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Input structure for WASM build_payload function
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmWitnessInput {
    secret_key: String,           // hex-encoded
    from: String,                 // 0x-prefixed hex address
    payment_nonce: String,        // 0x-prefixed hex (32 bytes)
    proving_key: String,          // hex-encoded or base64
    valid_after: u64,
    valid_before: u64,
}

/// Build a payment payload from requirements and witness data
///
/// # Arguments
/// * `requirements_json` - JSON string of PaymentRequirements (camelCase)
/// * `witness_json` - JSON string containing witness data and config
///
/// # Returns
/// JSON string of PaymentPayload (camelCase), or error message
///
/// # Example (JavaScript)
/// ```js
/// const requirements = {
///   scheme: "zk-settle",
///   network: "eip155:8453",
///   amount: "100",
///   asset: "0x...",
///   payTo: "0x...",
///   maxTimeoutSeconds: 3600,
///   extra: {
///     proofSystem: "groth16",
///     circuitId: "zk-settle-transfer-v1",
///     verifierAddress: "0x..."
///   }
/// };
///
/// const witness = {
///   secretKey: "0x...",
///   from: "0x...",
///   paymentNonce: "0x...",
///   provingKey: "0x...",
///   validAfter: 1700000000,
///   validBefore: 1700003600
/// };
///
/// const payload = build_payload(
///   JSON.stringify(requirements),
///   JSON.stringify(witness)
/// );
/// console.log(JSON.parse(payload));
/// ```
#[wasm_bindgen]
pub fn build_payload(requirements_json: &str, witness_json: &str) -> Result<String, JsValue> {
    // Parse requirements
    let requirements: zk402_core::PaymentRequirements = serde_json::from_str(requirements_json)
        .map_err(|e| JsValue::from_str(&format!("failed to parse requirements: {}", e)))?;

    // Parse witness input
    let witness_input: WasmWitnessInput = serde_json::from_str(witness_json)
        .map_err(|e| JsValue::from_str(&format!("failed to parse witness: {}", e)))?;

    // Decode hex fields
    let secret_key = decode_hex(&witness_input.secret_key)?;
    let from = decode_address(&witness_input.from)?;
    let payment_nonce = decode_nonce(&witness_input.payment_nonce)?;
    let proving_key = decode_hex(&witness_input.proving_key)?;

    // Build witness
    let witness = ClientWitness {
        secret_key,
        from,
        payment_nonce,
    };

    // Build config
    let config = ProverConfig {
        proving_key,
        valid_after: witness_input.valid_after,
        valid_before: witness_input.valid_before,
    };

    // Generate payload
    let payload = prover_build_payload(&requirements, &witness, &config)
        .map_err(|e| JsValue::from_str(&format!("proof generation failed: {}", e)))?;

    // Serialize to JSON
    serde_json::to_string(&payload)
        .map_err(|e| JsValue::from_str(&format!("failed to serialize payload: {}", e)))
}

/// Decode hex string (with or without 0x prefix)
fn decode_hex(s: &str) -> Result<Vec<u8>, JsValue> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).map_err(|e| JsValue::from_str(&format!("invalid hex: {}", e)))
}

/// Decode 20-byte address
fn decode_address(s: &str) -> Result<[u8; 20], JsValue> {
    let bytes = decode_hex(s)?;
    bytes
        .try_into()
        .map_err(|_| JsValue::from_str("address must be 20 bytes"))
}

/// Decode 32-byte nonce
fn decode_nonce(s: &str) -> Result<[u8; 32], JsValue> {
    let bytes = decode_hex(s)?;
    bytes
        .try_into()
        .map_err(|_| JsValue::from_str("nonce must be 32 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_hex_with_prefix() {
        let result = decode_hex("0x1234");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0x12, 0x34]);
    }

    #[test]
    fn decodes_hex_without_prefix() {
        let result = decode_hex("1234");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0x12, 0x34]);
    }

    #[test]
    fn decodes_address() {
        let addr = "0x0000000000000000000000000000000000000001";
        let result = decode_address(addr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[19], 1);
    }

    #[test]
    fn rejects_invalid_address_length() {
        let addr = "0x1234"; // Too short
        let result = decode_address(addr);
        assert!(result.is_err());
    }
}
