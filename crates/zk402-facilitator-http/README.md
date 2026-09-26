# zk402-facilitator-http

HTTP/REST API service for zk402 payment verification and settlement.

## Overview

This crate provides a production-ready HTTP server that exposes the zk402 facilitator functionality via REST endpoints. Built with Axum, it implements the standard x402 facilitator contract with `POST /verify` and `POST /settle` endpoints.

## Features

- **REST API** - Standard HTTP/JSON interface
- **x402 compatible** - Matches existing facilitator API contracts
- **Async runtime** - Built on Tokio for high concurrency
- **JSON serialization** - camelCase field names matching wire format
- **Health checks** - `/health` endpoint for monitoring
- **Graceful shutdown** - Clean termination on SIGTERM/SIGINT

## Endpoints

### `POST /verify`

Verifies a zero-knowledge payment proof.

**Request:**
```json
{
  "requirements": {
    "scheme": "zk-settle",
    "network": "eip155:8453",
    "amount": "1000000",
    "asset": "0x...",
    "payTo": "0x...",
    "maxTimeoutSeconds": 300,
    "extra": {
      "proofSystem": "groth16",
      "circuitId": "zk-settle-transfer-v1",
      "verifierAddress": "0x..."
    }
  },
  "payload": {
    "x402Version": 1,
    "accepted": { /* same as requirements */ },
    "proof": "0x...",
    "publicInputs": {
      "amount": "1000000",
      "asset": "0x...",
      "payTo": "0x...",
      "from": "0x...",
      "network": "eip155:8453",
      "nonce": "0x...",
      "validAfter": 1234567890,
      "validBefore": 1234568190,
      "publicKey": "0x..."
    }
  }
}
```

**Response:**
```json
{
  "isValid": true
}
```

Or on failure:
```json
{
  "isValid": false,
  "invalidReason": "proof verification failed"
}
```

### `POST /settle`

Settles a verified payment on-chain.

**Request:** Same as `/verify`

**Response:**
```json
{
  "success": true,
  "txHash": "0x..."
}
```

Or on failure:
```json
{
  "success": false,
  "error": "transaction reverted: nonce already consumed"
}
```

### `GET /health`

Health check endpoint.

**Response:**
```json
{
  "status": "ok"
}
```

## Running the Server

### From Source

```bash
cargo run --bin zk402-facilitator-http
```

### Configuration

Environment variables:

- `RPC_URL` - Ethereum JSON-RPC endpoint (required)
- `VERIFIER_ADDRESS` - On-chain verifier contract address (required)
- `FACILITATOR_PRIVATE_KEY` - Signing key for settlement transactions (required)
- `PORT` - HTTP server port (default: 3000)
- `HOST` - Bind address (default: 0.0.0.0)

Example:

```bash
export RPC_URL=https://mainnet.base.org
export VERIFIER_ADDRESS=0x1234567890123456789012345678901234567890
export FACILITATOR_PRIVATE_KEY=0xabcdef...
export PORT=8080

cargo run --bin zk402-facilitator-http
```

### Docker

```dockerfile
FROM rust:1.80 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin zk402-facilitator-http

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zk402-facilitator-http /usr/local/bin/
CMD ["zk402-facilitator-http"]
```

Build and run:

```bash
docker build -t zk402-facilitator .
docker run -p 3000:3000 \
  -e RPC_URL=https://mainnet.base.org \
  -e VERIFIER_ADDRESS=0x... \
  -e FACILITATOR_PRIVATE_KEY=0x... \
  zk402-facilitator
```

## Production Deployment

### Security Considerations

1. **Rate limiting** - Add a reverse proxy (nginx, Cloudflare) for DDoS protection
2. **TLS/HTTPS** - Terminate SSL at a load balancer or use Let's Encrypt
3. **Key management** - Store private keys in secure vaults (AWS Secrets Manager, HashiCorp Vault)
4. **Monitoring** - Track verification failures, settlement errors, and latency
5. **Logging** - Structured logs for debugging and audit trails

### Example nginx Configuration

```nginx
upstream facilitator {
    server localhost:3000;
}

server {
    listen 443 ssl http2;
    server_name facilitator.example.com;

    ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;

    location / {
        proxy_pass http://facilitator;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        limit_req zone=api burst=10;
    }
}
```

### Monitoring

Key metrics to track:

- Request rate (verify/settle per second)
- Verification success/failure ratio
- Settlement transaction success rate
- p50/p95/p99 latency
- RPC endpoint health
- Error rates by type

## Client Usage

```typescript
// Verify a payment
const verifyResponse = await fetch('https://facilitator.example.com/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ requirements, payload }),
});

const { isValid, invalidReason } = await verifyResponse.json();

if (isValid) {
  // Settle on-chain
  const settleResponse = await fetch('https://facilitator.example.com/settle', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ requirements, payload }),
  });

  const { success, txHash } = await settleResponse.json();
}
```

## Related Crates

- `zk402-facilitator` - Core facilitator logic
- `zk402-core` - Type definitions
- `axum` - HTTP framework
- `tokio` - Async runtime

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
