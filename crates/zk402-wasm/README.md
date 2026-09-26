# zk402-wasm

WASM bindings for zk402 client-side proof generation. Enables browser and Node.js clients to generate zero-knowledge payment proofs.

## Building

```bash
# Install wasm-pack if not already installed
cargo install wasm-pack

# Build for web
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs

# Build for bundlers (webpack, etc.)
wasm-pack build --target bundler
```

## Usage

### TypeScript/JavaScript

```typescript
import init, { build_payload } from './pkg/zk402_wasm';

await init();

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

const witness = {
  secretKey: "0x" + "00".repeat(32),
  from: "0x0000000000000000000000000000000000000004",
  paymentNonce: "0x" + "05".repeat(32),
  provingKey: "0x...", // Serialized Groth16 proving key
  validAfter: Math.floor(Date.now() / 1000),
  validBefore: Math.floor(Date.now() / 1000) + 3600
};

try {
  const payloadJson = build_payload(
    JSON.stringify(requirements),
    JSON.stringify(witness)
  );

  const payload = JSON.parse(payloadJson);
  console.log("Generated proof:", payload.proof);

  // Send to facilitator
  const response = await fetch('http://localhost:3000/verify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      requirements,
      payload
    })
  });

  const result = await response.json();
  console.log("Verification result:", result);
} catch (error) {
  console.error("Proof generation failed:", error);
}
```

## API

### `build_payload(requirements_json: string, witness_json: string): string`

Generates a zero-knowledge proof and assembles a complete `PaymentPayload`.

**Arguments:**
- `requirements_json`: JSON string of `PaymentRequirements` (camelCase)
- `witness_json`: JSON string containing:
  - `secretKey`: Hex-encoded private key (0x-prefixed)
  - `from`: Payer address (0x-prefixed, 20 bytes)
  - `paymentNonce`: Random nonce (0x-prefixed, 32 bytes)
  - `provingKey`: Hex-encoded Groth16 proving key
  - `validAfter`: Unix timestamp (seconds)
  - `validBefore`: Unix timestamp (seconds)

**Returns:**
- JSON string of `PaymentPayload` (camelCase), or throws error

**Example witness input:**
```json
{
  "secretKey": "0xabcd...",
  "from": "0x1234567890123456789012345678901234567890",
  "paymentNonce": "0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
  "provingKey": "0x...",
  "validAfter": 1700000000,
  "validBefore": 1700003600
}
```

## Error Handling

The function throws errors in these cases:
- Invalid JSON in either input
- Invalid hex encoding
- Invalid address/nonce length
- Proof generation failure

All errors are JavaScript `Error` objects with descriptive messages.

## Security Notes

- Never expose `secretKey` or `provingKey` in logs or error messages
- Generate `paymentNonce` cryptographically (e.g., `crypto.randomBytes(32)`)
- Always verify `validAfter` and `validBefore` are reasonable
- This module runs client-side - proof generation happens in the browser/Node process

## License

MIT OR Apache-2.0
