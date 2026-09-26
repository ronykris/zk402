//! zk402 HTTP facilitator server
//!
//! Runs the HTTP service on port 3000 (configurable via PORT env var).

use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zk402_facilitator::{StaticRegistry, ZkSettleFacilitator};
use zk402_facilitator_http::app_router;
use zk402_groth16::proof_system::Groth16ProofSystem;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "zk402_facilitator_http=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create a static registry (in production, load from config)
    let registry = StaticRegistry::new();

    // TODO: Load verifying keys from configuration
    // For now, this is an empty registry - would need setup() from zk402-groth16
    // registry.register("groth16".to_string(), "zk-settle-transfer-v1".to_string(), vk);

    // Create facilitator
    let facilitator = Arc::new(ZkSettleFacilitator::<Groth16ProofSystem, _>::new(registry));

    // Build router
    let app = app_router(facilitator);

    // Determine port
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("zk402 facilitator listening on {}", addr);

    // Run server
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
