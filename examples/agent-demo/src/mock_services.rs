//! Mock API services that accept zk402 payments
//!
//! This demonstrates services returning 402 Payment Required
//! and accepting zero-knowledge payment proofs

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use zk402_core::{PaymentRequirements, PaymentPayload, ZkExtra};

#[derive(Debug, Serialize, Deserialize)]
struct SearchRequest {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResponse {
    papers: Vec<Paper>,
    payment_received: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Paper {
    title: String,
    authors: Vec<String>,
    abstract_text: String,
    year: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisRequest {
    papers: Vec<Paper>,
    task: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisResponse {
    insights: Vec<String>,
    summary: String,
    payment_received: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VisualizationRequest {
    data: serde_json::Value,
    chart_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VisualizationResponse {
    charts: Vec<String>,
    payment_received: Option<String>,
}

/// Helper to check if payment is provided in headers
fn extract_payment(headers: &axum::http::HeaderMap) -> Option<PaymentPayload> {
    headers
        .get("x-payment")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| serde_json::from_str(s).ok())
}

/// Verify a payment (simplified - in production would verify proof)
fn verify_payment(
    payload: &PaymentPayload,
    expected_amount: &str,
) -> Result<String, String> {
    // In a real implementation, you would:
    // 1. Verify the proof cryptographically
    // 2. Check the amount matches
    // 3. Verify time windows
    // 4. Check nonce hasn't been used
    // 5. Submit to blockchain for settlement

    if payload.public_inputs.amount != expected_amount {
        return Err(format!(
            "Payment amount mismatch: expected {}, got {}",
            expected_amount, payload.public_inputs.amount
        ));
    }

    // Simulate successful settlement
    let tx_hash = format!("0x{}", hex::encode(&payload.public_inputs.nonce));
    Ok(tx_hash)
}

/// Academic paper search service - costs $0.50 USDC
async fn paper_search(
    headers: axum::http::HeaderMap,
    Json(req): Json<SearchRequest>,
) -> Response {
    println!("📚 Paper search request: {}", req.query);

    // Check for payment
    let payment = extract_payment(&headers);

    if payment.is_none() {
        // Return 402 Payment Required
        println!("   ❌ No payment provided, returning 402");

        let requirements = PaymentRequirements {
            scheme: "zk-settle".to_string(),
            network: "eip155:8453".to_string(),
            amount: "500000".to_string(), // $0.50 USDC (6 decimals)
            asset: [0x83; 20], // Mock USDC address
            pay_to: [0x01; 20], // Service wallet
            max_timeout_seconds: 300,
            extra: ZkExtra {
                proof_system: "groth16".to_string(),
                circuit_id: "zk-settle-transfer-v1".to_string(),
                verifier_address: [0x12; 20],
            },
        };

        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "error": "Payment Required",
                "requirements": requirements
            })),
        )
            .into_response();
    }

    // Verify payment
    let payload = payment.unwrap();
    match verify_payment(&payload, "500000") {
        Ok(tx_hash) => {
            println!("   ✅ Payment verified: {}", tx_hash);

            // Return mock papers
            let papers = vec![
                Paper {
                    title: format!("Advances in {}", req.query),
                    authors: vec!["Dr. Alice Smith".to_string(), "Dr. Bob Jones".to_string()],
                    abstract_text: "This groundbreaking research explores...".to_string(),
                    year: 2024,
                },
                Paper {
                    title: format!("{} and Machine Learning", req.query),
                    authors: vec!["Prof. Carol White".to_string()],
                    abstract_text: "We present novel approaches to...".to_string(),
                    year: 2024,
                },
                Paper {
                    title: format!("A Survey of {}", req.query),
                    authors: vec!["Dr. David Brown".to_string(), "Dr. Eve Green".to_string()],
                    abstract_text: "This comprehensive survey covers...".to_string(),
                    year: 2023,
                },
            ];

            (
                StatusCode::OK,
                Json(SearchResponse {
                    papers: papers.into_iter().take(req.limit.unwrap_or(10)).collect(),
                    payment_received: Some(tx_hash),
                }),
            )
                .into_response()
        }
        Err(e) => {
            println!("   ❌ Payment verification failed: {}", e);
            (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({"error": "Payment verification failed", "reason": e})),
            )
                .into_response()
        }
    }
}

/// AI analysis service - costs $2.00 USDC
async fn analyze_papers(
    headers: axum::http::HeaderMap,
    Json(req): Json<AnalysisRequest>,
) -> Response {
    println!("🧠 Analysis request for {} papers", req.papers.len());

    let payment = extract_payment(&headers);

    if payment.is_none() {
        println!("   ❌ No payment provided, returning 402");

        let requirements = PaymentRequirements {
            scheme: "zk-settle".to_string(),
            network: "eip155:8453".to_string(),
            amount: "2000000".to_string(), // $2.00 USDC
            asset: [0x83; 20],
            pay_to: [0x02; 20],
            max_timeout_seconds: 300,
            extra: ZkExtra {
                proof_system: "groth16".to_string(),
                circuit_id: "zk-settle-transfer-v1".to_string(),
                verifier_address: [0x12; 20],
            },
        };

        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "error": "Payment Required",
                "requirements": requirements
            })),
        )
            .into_response();
    }

    let payload = payment.unwrap();
    match verify_payment(&payload, "2000000") {
        Ok(tx_hash) => {
            println!("   ✅ Payment verified: {}", tx_hash);

            let insights = vec![
                "Common theme: rapid advancement in the field".to_string(),
                "Key finding: convergence of multiple approaches".to_string(),
                "Future direction: integration with quantum computing".to_string(),
            ];

            (
                StatusCode::OK,
                Json(AnalysisResponse {
                    insights: insights.clone(),
                    summary: format!(
                        "Analysis of {} papers reveals {}",
                        req.papers.len(),
                        insights.join(", ")
                    ),
                    payment_received: Some(tx_hash),
                }),
            )
                .into_response()
        }
        Err(e) => {
            println!("   ❌ Payment verification failed: {}", e);
            (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({"error": "Payment verification failed", "reason": e})),
            )
                .into_response()
        }
    }
}

/// Visualization service - costs $1.00 USDC
async fn generate_charts(
    headers: axum::http::HeaderMap,
    Json(req): Json<VisualizationRequest>,
) -> Response {
    println!("📊 Visualization request: {}", req.chart_type);

    let payment = extract_payment(&headers);

    if payment.is_none() {
        println!("   ❌ No payment provided, returning 402");

        let requirements = PaymentRequirements {
            scheme: "zk-settle".to_string(),
            network: "eip155:8453".to_string(),
            amount: "1000000".to_string(), // $1.00 USDC
            asset: [0x83; 20],
            pay_to: [0x03; 20],
            max_timeout_seconds: 300,
            extra: ZkExtra {
                proof_system: "groth16".to_string(),
                circuit_id: "zk-settle-transfer-v1".to_string(),
                verifier_address: [0x12; 20],
            },
        };

        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "error": "Payment Required",
                "requirements": requirements
            })),
        )
            .into_response();
    }

    let payload = payment.unwrap();
    match verify_payment(&payload, "1000000") {
        Ok(tx_hash) => {
            println!("   ✅ Payment verified: {}", tx_hash);

            (
                StatusCode::OK,
                Json(VisualizationResponse {
                    charts: vec![
                        "trend_chart.png".to_string(),
                        "distribution_chart.png".to_string(),
                        "comparison_chart.png".to_string(),
                    ],
                    payment_received: Some(tx_hash),
                }),
            )
                .into_response()
        }
        Err(e) => {
            println!("   ❌ Payment verification failed: {}", e);
            (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({"error": "Payment verification failed", "reason": e})),
            )
                .into_response()
        }
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 Starting mock x402 services...\n");

    let app = Router::new()
        .route("/api/search", post(paper_search))
        .route("/api/analyze", post(analyze_papers))
        .route("/api/visualize", post(generate_charts));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    println!("📡 Services running on http://127.0.0.1:8080");
    println!("   POST /api/search     - $0.50 USDC");
    println!("   POST /api/analyze    - $2.00 USDC");
    println!("   POST /api/visualize  - $1.00 USDC");
    println!("\nWaiting for agent requests...\n");

    axum::serve(listener, app).await.unwrap();
}
