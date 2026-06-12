# lnurl-server

A simple LNURL pay server. Allows you to have a lightning address for your own node.

## Installation

```bash
cargo install lnurl-server
```

## Usage

```bash
lnurl-server --domain mydomain.com --network bitcoin --data-dir ~/.lnurl-server/ --port 8080 --lnd-host localhost --lnd-port 10009 --macaroon-file ~/.lnd/data/chain/bitcoin/mainnet/admin.macaroon --cert-file ~/.lnd/tls.cert
```

By default the crate builds with both `lnd` and `ldk-server` support and uses LND unless `--node-backend ldk-server` is set. At least one backend feature must be enabled.

To use LDK Server:

```bash
lnurl-server --node-backend ldk-server --domain mydomain.com --network bitcoin --data-dir ~/.lnurl-server/ --port 8080 --ldk-server-host localhost --ldk-server-port 3536 --ldk-server-cert-file ~/.ldk-server/tls.crt --ldk-server-api-key-file ~/.ldk-server/bitcoin/api_key
```

For backend-specific builds:

```bash
cargo build --no-default-features --features lnd
cargo build --no-default-features --features ldk-server
```

## Testing with Curl and LNURL-Pay Protocol

### LNURL-Pay Protocol Overview

The lnurl-pay flow consists of 2 main steps:

1. **Initial Request**: `GET /.well-known/lnurlp/{name}` → returns callback URL and payment parameters
2. **Invoice Request**: `GET /get-invoice/{hash}?amount={amount}` → returns Lightning invoice

### Testing Steps

#### 1. Health Check

Verify the server is running:

```bash
curl http://localhost:3000/health-check
```

#### 2. Get Payment Details (Step 1)

Request payment metadata and callback information for a Lightning address:

```bash
curl http://localhost:3000/.well-known/lnurlp/satoshi
```

**Expected Response:**
```json
{
  "callback": "https://yourdomain.com/get-invoice/[hash]",
  "minSendable": 1000,
  "maxSendable": 100000000,
  "tag": "payRequest",
  "metadata": "[[\"text/identifier\",\"satoshi@yourdomain.com\"],[\"text/plain\",\"Sats for satoshi\"]]",
  "allowsNostr": true,
  "nostrPubkey": "npub1..."
}
```

**Response Fields:**
- `callback`: URL to request invoice from (use in step 2)
- `minSendable`: Minimum amount in millisatoshis
- `maxSendable`: Maximum amount in millisatoshis
- `metadata`: Serialized JSON array with payment description
- `allowsNostr`: Whether Nostr zap requests are supported

#### 3. Request Invoice (Step 2)

Using the callback URL from step 1, request a Lightning invoice with a specific amount:

```bash
# Request 50000 millisatoshis (0.05 sats)
curl "https://yourdomain.com/get-invoice/[hash]?amount=50000"
```

**Expected Response:**
```json
{
  "pr": "lnbc500n1p3h...",
  "routes": [],
  "successAction": {
    "tag": "message",
    "message": "Payment received!"
  }
}
```

**Response Fields:**
- `pr`: BOLT11-serialized Lightning invoice
- `routes`: Route hints (empty array in most cases)
- `successAction`: Optional success message shown after payment (LUD-09)

#### 4. Verify Payment Status (Optional)

Check if an invoice has been paid:

```bash
curl "http://localhost:3000/verify/[desc_hash]/[payment_hash]"
```

### Complete Test Workflow

Here's a complete workflow demonstrating the full lnurl-pay flow:

```bash
#!/bin/bash

# Step 1: Get payment details for user "alice"
echo "Step 1: Getting payment details..."
RESPONSE=$(curl -s http://localhost:3000/.well-known/lnurlp/alice)
echo "$RESPONSE" | jq .

# Extract callback URL from response
CALLBACK=$(echo "$RESPONSE" | jq -r '.callback')
echo "Callback URL: $CALLBACK"

# Step 2: Request an invoice for 50000 millisatoshis (0.05 sats)
echo -e "\nStep 2: Requesting invoice..."
AMOUNT=50000
INVOICE_RESPONSE=$(curl -s "$CALLBACK?amount=$AMOUNT")
echo "$INVOICE_RESPONSE" | jq .

# Extract invoice
PR=$(echo "$INVOICE_RESPONSE" | jq -r '.pr')
echo "Invoice to pay: $PR"

# Step 3: Pay the invoice with your Lightning wallet
echo -e "\nStep 3: Pay the invoice with your Lightning wallet"
```

### Key Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `name` | Path | Username in Lightning address | `alice`, `bob` |
| `amount` | Query | Amount in millisatoshis | `10000` (0.01 sats) |
| `minSendable` | Response | Minimum amount server accepts | `1000` |
| `maxSendable` | Response | Maximum amount server accepts | `100000000` |

### Common Issues

**Missing amount parameter:** Add `?amount=AMOUNT` to the callback URL
```bash
# ❌ Wrong
curl "https://yourdomain.com/get-invoice/[hash]"

# ✅ Correct
curl "https://yourdomain.com/get-invoice/[hash]?amount=50000"
```

**Domain mismatch:** Ensure the `DOMAIN` configuration matches your server hostname
```bash
# In your config
DOMAIN=yourdomain.com
```

**Invalid hash:** Use the callback URL exactly as returned from the first request
```bash
RESPONSE=$(curl -s http://localhost:3000/.well-known/lnurlp/alice)
CALLBACK=$(echo "$RESPONSE" | jq -r '.callback')
curl "$CALLBACK?amount=50000"
```

### Testing Different Amount Ranges

```bash
# Minimum amount (1 satoshi)
curl "https://yourdomain.com/get-invoice/[hash]?amount=1000"

# Small amount (0.1 sats)
curl "https://yourdomain.com/get-invoice/[hash]?amount=100000"

# Medium amount (1 sat)
curl "https://yourdomain.com/get-invoice/[hash]?amount=1000000"

# Large amount (100 sats)
curl "https://yourdomain.com/get-invoice/[hash]?amount=100000000"
```

## Specification References

- **LUD-06**: `payRequest` base spec - https://github.com/fiatjaf/lnurl-rfc/blob/luds/06.md
- **LUD-09**: Success action - https://github.com/fiatjaf/lnurl-rfc/blob/luds/09.md
- **BOLT11**: Lightning invoice format - https://github.com/lightningnetwork/lightning-rfc/blob/master/11-payment-encoding.md
