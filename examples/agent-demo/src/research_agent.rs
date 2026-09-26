//! Autonomous Research Agent
//!
//! This agent autonomously:
//! 1. Discovers services it needs
//! 2. Reads payment requirements (x402)
//! 3. Generates zero-knowledge proofs
//! 4. Pays for services
//! 5. Completes its research task

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use zk402_core::{PaymentPayload, PaymentRequirements, PublicInputs, Witness};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Paper {
    title: String,
    authors: Vec<String>,
    abstract_text: String,
    year: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResponse {
    papers: Vec<Paper>,
    payment_received: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisResponse {
    insights: Vec<String>,
    summary: String,
    payment_received: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VisualizationResponse {
    charts: Vec<String>,
    payment_received: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaymentRequiredResponse {
    error: String,
    requirements: PaymentRequirements,
}

struct AutonomousAgent {
    wallet_key: Vec<u8>,
    wallet_address: [u8; 20],
    budget_usdc: f64,
    spent_usdc: f64,
    client: Client,
}

impl AutonomousAgent {
    fn new(budget_usdc: f64) -> Self {
        // In production, load from secure storage
        let wallet_key = vec![0xab; 32];
        let wallet_address = [0xaa; 20];

        Self {
            wallet_key,
            wallet_address,
            budget_usdc,
            spent_usdc: 0.0,
            client: Client::new(),
        }
    }

    /// Main agent task: research a topic
    async fn research_topic(&mut self, topic: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🤖 Autonomous Research Agent");
        println!("{}", "=".repeat(50));
        println!("📋 Task: Research '{}'", topic);
        println!("💰 Budget: ${:.2} USDC", self.budget_usdc);
        println!("📍 Wallet: 0x{}", hex::encode(&self.wallet_address));
        println!();

        // Step 1: Search for papers
        println!("Step 1: Searching for academic papers...");
        let papers = self
            .call_service(
                "http://127.0.0.1:8080/api/search",
                json!({
                    "query": topic,
                    "limit": 5
                }),
            )
            .await?;

        let search_response: SearchResponse = serde_json::from_value(papers)?;
        println!("   ✅ Found {} papers", search_response.papers.len());
        for paper in &search_response.papers {
            println!("      - {}", paper.title);
        }
        println!();

        // Step 2: Analyze the papers
        println!("Step 2: Running AI analysis...");
        let analysis = self
            .call_service(
                "http://127.0.0.1:8080/api/analyze",
                json!({
                    "papers": search_response.papers,
                    "task": "extract_insights"
                }),
            )
            .await?;

        let analysis_response: AnalysisResponse = serde_json::from_value(analysis)?;
        println!("   ✅ Analysis complete");
        println!("      Summary: {}", analysis_response.summary);
        for insight in &analysis_response.insights {
            println!("      • {}", insight);
        }
        println!();

        // Step 3: Generate visualizations
        println!("Step 3: Generating visualizations...");
        let viz = self
            .call_service(
                "http://127.0.0.1:8080/api/visualize",
                json!({
                    "data": { "insights": analysis_response.insights },
                    "chart_type": "interactive"
                }),
            )
            .await?;

        let viz_response: VisualizationResponse = serde_json::from_value(viz)?;
        println!("   ✅ Generated {} charts", viz_response.charts.len());
        for chart in &viz_response.charts {
            println!("      - {}", chart);
        }
        println!();

        // Final summary
        println!("{}", "=".repeat(50));
        println!("✅ Research Complete!");
        println!("💸 Total spent: ${:.2} USDC", self.spent_usdc);
        println!("💰 Remaining budget: ${:.2} USDC", self.budget_usdc - self.spent_usdc);
        println!();

        Ok(())
    }

    /// Call a service, automatically handling 402 payments
    async fn call_service(
        &mut self,
        url: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // First attempt without payment
        let response = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await?;

        // Check if payment is required
        if response.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
            let payment_info: PaymentRequiredResponse = response.json().await?;
            let requirements = payment_info.requirements;

            let cost_usdc = requirements.amount.parse::<f64>()? / 1_000_000.0;
            println!("   💳 Payment required: ${:.2} USDC", cost_usdc);

            // Check budget
            if self.spent_usdc + cost_usdc > self.budget_usdc {
                return Err(format!(
                    "Insufficient budget! Need ${:.2} but only ${:.2} remaining",
                    cost_usdc,
                    self.budget_usdc - self.spent_usdc
                )
                .into());
            }

            // Generate payment proof
            println!("   🔐 Generating zero-knowledge proof...");
            let payload = self.generate_payment_proof(&requirements)?;

            // Retry with payment
            let response = self
                .client
                .post(url)
                .header("X-Payment", serde_json::to_string(&payload)?)
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(format!("Payment failed: {}", error_text).into());
            }

            // Track spending
            self.spent_usdc += cost_usdc;
            println!("   ✅ Payment successful (${:.2} USDC)", cost_usdc);

            let result = response.json().await?;
            return Ok(result);
        }

        // No payment needed
        Ok(response.json().await?)
    }

    /// Generate a zero-knowledge payment proof
    fn generate_payment_proof(
        &self,
        requirements: &PaymentRequirements,
    ) -> Result<PaymentPayload, Box<dyn std::error::Error>> {
        // Generate a random nonce
        let nonce: [u8; 32] = rand::random();

        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create witness (private data)
        let _witness = Witness {
            secret_key: self.wallet_key.clone(),
            nonce_scalar: nonce.to_vec(),
        };

        // Create public inputs
        let public_inputs = PublicInputs {
            amount: requirements.amount.clone(),
            asset: requirements.asset,
            pay_to: requirements.pay_to,
            from: self.wallet_address,
            network: requirements.network.clone(),
            nonce,
            valid_after: now,
            valid_before: now + requirements.max_timeout_seconds,
            public_key: vec![0xbb; 32], // Would derive from secret_key in production
        };

        // In production, would generate real proof using zk402-prover
        // For demo, create mock payload
        let payload = PaymentPayload {
            x402_version: 1,
            accepted: requirements.clone(),
            proof: vec![0xab; 192], // Mock Groth16 proof (192 bytes)
            public_inputs,
            proof_system_extra: None,
        };

        Ok(payload)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create agent with $10 budget
    let mut agent = AutonomousAgent::new(10.0);

    // Run autonomous research
    agent.research_topic("quantum computing").await?;

    println!("🎯 Agent completed its task autonomously!");
    println!("   No human intervention was required.");
    println!("   Agent discovered services, paid for them, and delivered results.");

    Ok(())
}
