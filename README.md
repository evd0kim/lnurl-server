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
