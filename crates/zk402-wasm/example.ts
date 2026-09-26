/**
 * Example TypeScript usage of zk402-wasm
 *
 * To run:
 * 1. Build WASM: wasm-pack build --target nodejs
 * 2. Install: npm install
 * 3. Run: node example.js (after compiling this file)
 */

import init, { build_payload } from './pkg/zk402_wasm';

async function main() {
  // Initialize WASM module
  await init();

  // Payment requirements from payee
  const requirements = {
    scheme: "zk-settle",
    network: "eip155:8453",
    amount: "100",
    asset: "0x0000000000000000000000000000000000000001",
    payTo: "0x0000000000000000000000000000000000000002",
    maxTimeoutSeconds: 3600,
    extra: {
      proofSystem: "groth16",
      circuitId: "zk-settle-transfer-v1",
      verifierAddress: "0x0000000000000000000000000000000000000003"
    }
  };

  // Client witness (keep secret!)
  const witness = {
    secretKey: "0x" + "aa".repeat(32),  // 32-byte private key
    from: "0x0000000000000000000000000000000000000004",
    paymentNonce: "0x" + Buffer.from(crypto.randomBytes(32)).toString('hex'),
    provingKey: "0x" + "bb".repeat(100),  // Groth16 proving key (would be real in production)
    validAfter: Math.floor(Date.now() / 1000),
    validBefore: Math.floor(Date.now() / 1000) + 3600
  };

  try {
    console.log("Generating proof...");
    const payloadJson = build_payload(
      JSON.stringify(requirements),
      JSON.stringify(witness)
    );

    const payload = JSON.parse(payloadJson);

    console.log("✓ Proof generated successfully");
    console.log("  Proof length:", payload.proof.length);
    console.log("  Public inputs:", {
      amount: payload.publicInputs.amount,
      from: payload.publicInputs.from,
      payTo: payload.publicInputs.payTo,
      network: payload.publicInputs.network
    });

    // Send to facilitator for verification
    console.log("\nSending to facilitator...");
    const response = await fetch('http://localhost:3000/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        requirements,
        payload
      })
    });

    const result = await response.json();

    if (result.isValid) {
      console.log("✓ Payment verified by facilitator");

      // Proceed to settlement
      const settleResponse = await fetch('http://localhost:3000/settle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          requirements,
          payload
        })
      });

      const settleResult = await settleResponse.json();

      if (settleResult.success) {
        console.log("✓ Payment settled");
        console.log("  Transaction hash:", settleResult.txHash);
      } else {
        console.error("✗ Settlement failed:", settleResult.error);
      }
    } else {
      console.error("✗ Verification failed:", result.invalidReason);
    }

  } catch (error) {
    console.error("Error:", error);
    process.exit(1);
  }
}

main();
