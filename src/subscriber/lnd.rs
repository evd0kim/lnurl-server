use crate::subscriber::spawn_paid_invoice_handler;
use bitcoin::hashes::sha256;
use nostr::Keys;
use sled::Db;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic_openssl_lnd::lnrpc::invoice::InvoiceState;
use tonic_openssl_lnd::{lnrpc, LndLightningClient};

pub async fn start(
    db: Db,
    mut lnd: LndLightningClient,
    key: Keys,
    telegram_token: Option<String>,
    telegram_id: Option<String>,
    name_watcher: Arc<RwLock<HashMap<sha256::Hash, String>>>,
) {
    let client = reqwest::Client::new();
    loop {
        println!("Starting invoice subscription");

        let sub = lnrpc::InvoiceSubscription::default();
        let mut invoice_stream = lnd
            .subscribe_invoices(sub)
            .await
            .expect("Failed to start invoice subscription")
            .into_inner();

        while let Some(ln_invoice) = invoice_stream
            .message()
            .await
            .expect("Failed to receive invoices")
        {
            match InvoiceState::from_i32(ln_invoice.state) {
                Some(InvoiceState::Settled) => {
                    spawn_paid_invoice_handler(
                        db.clone(),
                        hex::encode(ln_invoice.r_hash),
                        key.clone(),
                        client.clone(),
                        telegram_token.clone(),
                        telegram_id.clone(),
                        ln_invoice.description_hash,
                        Arc::clone(&name_watcher),
                    );
                }
                None
                | Some(InvoiceState::Canceled)
                | Some(InvoiceState::Open)
                | Some(InvoiceState::Accepted) => {}
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
