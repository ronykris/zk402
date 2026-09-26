//! HTTP facilitator service exposing verify and settle endpoints
//!
//! Implements the x402 facilitator HTTP contract with POST /verify and POST /settle.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use zk402_core::*;
use zk402_facilitator::ZkSettleFacilitator;

/// HTTP request for POST /verify
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRequest {
    pub requirements: PaymentRequirements,
    pub payload: PaymentPayload,
}

/// HTTP request for POST /settle
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleRequest {
    pub requirements: PaymentRequirements,
    pub payload: PaymentPayload,
}

/// Application state holding the facilitator instance
pub struct AppState<PS: ProofSystem, VKR: VerifyingKeyRegistry> {
    pub facilitator: Arc<ZkSettleFacilitator<PS, VKR>>,
}

/// Build the HTTP router with verify and settle endpoints
pub fn app_router<PS, VKR>(facilitator: Arc<ZkSettleFacilitator<PS, VKR>>) -> Router
where
    PS: ProofSystem + Send + Sync + 'static,
    VKR: VerifyingKeyRegistry<VerifyingKeyHandle = PS::VerifyingKey> + Send + Sync + 'static,
{
    let state = Arc::new(AppState { facilitator });

    Router::new()
        .route("/verify", post(verify_handler::<PS, VKR>))
        .route("/settle", post(settle_handler::<PS, VKR>))
        .with_state(state)
}

/// POST /verify handler
async fn verify_handler<PS, VKR>(
    State(state): State<Arc<AppState<PS, VKR>>>,
    Json(request): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError>
where
    PS: ProofSystem + Send + Sync,
    VKR: VerifyingKeyRegistry<VerifyingKeyHandle = PS::VerifyingKey> + Send + Sync,
{
    let response = state
        .facilitator
        .verify(&request.requirements, &request.payload)
        .await
        .map_err(AppError::from)?;

    Ok(Json(response))
}

/// POST /settle handler
async fn settle_handler<PS, VKR>(
    State(state): State<Arc<AppState<PS, VKR>>>,
    Json(request): Json<SettleRequest>,
) -> Result<Json<SettleResponse>, AppError>
where
    PS: ProofSystem + Send + Sync,
    VKR: VerifyingKeyRegistry<VerifyingKeyHandle = PS::VerifyingKey> + Send + Sync,
{
    let response = state
        .facilitator
        .settle(&request.requirements, &request.payload)
        .await
        .map_err(AppError::from)?;

    Ok(Json(response))
}

/// Application error wrapper
#[derive(Debug)]
pub struct AppError(SchemeError);

impl From<SchemeError> for AppError {
    fn from(err: SchemeError) -> Self {
        Self(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            SchemeError::StructuralMismatch(msg) => (StatusCode::BAD_REQUEST, msg),
            SchemeError::UnsupportedCircuit => {
                (StatusCode::BAD_REQUEST, "unsupported circuit".to_string())
            }
            SchemeError::CryptographicVerificationFailed => {
                (StatusCode::BAD_REQUEST, "proof verification failed".to_string())
            }
            SchemeError::Expired => {
                (StatusCode::BAD_REQUEST, "payment window expired".to_string())
            }
            SchemeError::ChainError(msg) => (StatusCode::BAD_GATEWAY, msg),
            SchemeError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;
    use zk402_facilitator::StaticRegistry;

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
            Ok(proof == &[0x42])
        }
    }

    fn test_facilitator() -> Arc<ZkSettleFacilitator<MockProofSystem, StaticRegistry<Vec<u8>>>> {
        let mut registry = StaticRegistry::new();
        registry.register(
            "groth16".to_string(),
            "zk-settle-transfer-v1".to_string(),
            vec![0u8],
        );
        Arc::new(ZkSettleFacilitator::new(registry))
    }

    fn test_payload() -> PaymentPayload {
        PaymentPayload {
            x402_version: 1,
            accepted: PaymentRequirements {
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
            },
            proof: vec![0x42],
            public_inputs: PublicInputs {
                amount: "100".to_string(),
                asset: [1u8; 20],
                pay_to: [2u8; 20],
                from: [4u8; 20],
                network: "eip155:8453".to_string(),
                nonce: [5u8; 32],
                valid_after: 1000000000,
                valid_before: 9999999999,
                public_key: vec![6u8; 32],
            },
            proof_system_extra: None,
        }
    }

    #[tokio::test]
    async fn verify_endpoint_returns_valid_response() {
        let app = app_router(test_facilitator());
        let payload = test_payload();

        let request = Request::builder()
            .uri("/verify")
            .method("POST")
            .header("content-type", "application/json")
            .body(
                serde_json::to_string(&VerifyRequest {
                    requirements: payload.accepted.clone(),
                    payload: payload.clone(),
                })
                .unwrap(),
            )
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn verify_endpoint_rejects_invalid_proof() {
        let app = app_router(test_facilitator());
        let mut payload = test_payload();
        payload.proof = vec![0x99]; // Invalid proof

        let request = Request::builder()
            .uri("/verify")
            .method("POST")
            .header("content-type", "application/json")
            .body(
                serde_json::to_string(&VerifyRequest {
                    requirements: payload.accepted.clone(),
                    payload: payload.clone(),
                })
                .unwrap(),
            )
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
