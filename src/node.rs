use crate::config::Config;
#[cfg(any(feature = "lnd", feature = "ldk-server", feature = "phoenixd"))]
use crate::config::NodeBackend;
use anyhow::{anyhow, bail};
use bitcoin::hashes::sha256;
#[cfg(any(feature = "lnd", feature = "ldk-server", feature = "phoenixd"))]
use bitcoin::hashes::Hash;
use lightning_invoice::Bolt11Invoice;
use std::str::FromStr;
#[cfg(feature = "ldk-server")]
use std::sync::Arc;

#[cfg(feature = "ldk-server")]
use ldk_server_client::client::LdkServerClient;
#[cfg(feature = "ldk-server")]
use ldk_server_client::ldk_server_grpc::api::{
    Bolt11ReceiveRequest, GetNodeInfoRequest, GetPaymentDetailsRequest,
};
#[cfg(feature = "ldk-server")]
use ldk_server_client::ldk_server_grpc::types::{
    bolt11_invoice_description, payment_kind, Payment, PaymentStatus,
};

#[cfg(feature = "lnd")]
use tonic_openssl_lnd::lnrpc;
#[cfg(feature = "lnd")]
use tonic_openssl_lnd::lnrpc::invoice::InvoiceState;
#[cfg(feature = "lnd")]
use tonic_openssl_lnd::lnrpc::{GetInfoRequest, GetInfoResponse};
#[cfg(feature = "lnd")]
use tonic_openssl_lnd::LndLightningClient;

#[cfg(feature = "phoenixd")]
use phoenixd_rs::Phoenixd;

#[derive(Clone)]
pub enum NodeClient {
    #[cfg(feature = "lnd")]
    Lnd(LndLightningClient),
    #[cfg(feature = "ldk-server")]
    LdkServer(Arc<LdkServerClient>),
    #[cfg(feature = "phoenixd")]
    Phoenixd(Phoenixd),
}

pub struct CreatedInvoice {
    pub payment_request: String,
    pub payment_hash: String,
}

pub struct InvoiceStatus {
    pub payment_request: Option<String>,
    pub settled: bool,
    pub preimage: Option<String>,
}

impl NodeClient {
    pub async fn connect(config: &Config) -> anyhow::Result<Self> {
        match config.node_backend() {
            #[cfg(feature = "lnd")]
            NodeBackend::Lnd => {
                let mut client = tonic_openssl_lnd::connect(
                    config.lnd_host.clone(),
                    config.lnd_port,
                    config.cert_file(),
                    config.macaroon_file(),
                )
                .await
                .expect("failed to connect");

                let mut ln_client = client.lightning().clone();
                let lnd_info: GetInfoResponse = ln_client
                    .get_info(GetInfoRequest {})
                    .await
                    .expect("Failed to get lnd info")
                    .into_inner();

                println!("Connected to LND: {}", lnd_info.identity_pubkey);
                Ok(Self::Lnd(client.lightning().clone()))
            }
            #[cfg(feature = "ldk-server")]
            NodeBackend::LdkServer => {
                let cert_pem = std::fs::read(config.ldk_server_cert_file())?;
                let api_key = std::fs::read_to_string(config.ldk_server_api_key_file())?
                    .trim()
                    .to_string();
                let client = LdkServerClient::new(
                    format!("{}:{}", config.ldk_server_host, config.ldk_server_port),
                    api_key,
                    &cert_pem,
                )
                .map_err(|e| anyhow!("Failed to create ldk-server client: {e}"))?;
                let info = client.get_node_info(GetNodeInfoRequest {}).await?;

                println!("Connected to LDK Server: {}", info.node_id);
                Ok(Self::LdkServer(Arc::new(client)))
            }
            #[cfg(feature = "phoenixd")]
            NodeBackend::Phoenixd => {
                let url = config.phoenixd_url.clone()
                    .ok_or_else(|| anyhow!("LNURL_PHOENIXD_URL must be set for Phoenixd backend"))?;
                let api_password = config.phoenixd_api_key.clone()
                    .ok_or_else(|| anyhow!("LNURL_PHOENIXD_API_KEY must be set for Phoenixd backend"))?;

                let client = Phoenixd::new(&api_password, &url)?;
                println!("Connected to Phoenixd");
                Ok(Self::Phoenixd(client))
            }
            #[allow(unreachable_patterns)]
            backend => bail!("Backend {backend:?} is not enabled at compile time"),
        }
    }

    pub async fn create_invoice(
        &self,
        desc_hash: sha256::Hash,
        amount_msats: u64,
        _route_hints: bool,
    ) -> anyhow::Result<CreatedInvoice> {
        match self {
            #[cfg(feature = "lnd")]
            Self::Lnd(lnd) => {
                let mut lnd = lnd.clone();
                let request = lnrpc::Invoice {
                    value_msat: amount_msats as i64,
                    description_hash: desc_hash.to_byte_array().to_vec(),
                    expiry: 86_400,
                    private: _route_hints,
                    ..Default::default()
                };
                let resp = lnd.add_invoice(request).await?.into_inner();

                Ok(CreatedInvoice {
                    payment_request: resp.payment_request,
                    payment_hash: hex::encode(resp.r_hash),
                })
            }
            #[cfg(feature = "ldk-server")]
            Self::LdkServer(client) => {
                let description =
                    ldk_server_client::ldk_server_grpc::types::Bolt11InvoiceDescription {
                        kind: Some(bolt11_invoice_description::Kind::Hash(hex::encode(
                            desc_hash.to_byte_array(),
                        ))),
                    };
                let resp = client
                    .bolt11_receive(Bolt11ReceiveRequest {
                        amount_msat: Some(amount_msats),
                        description: Some(description),
                        expiry_secs: 86_400,
                    })
                    .await?;

                Ok(CreatedInvoice {
                    payment_request: resp.invoice,
                    payment_hash: resp.payment_hash,
                })
            }
            #[cfg(feature = "phoenixd")]
            Self::Phoenixd(client) => {
                let invoice_req = phoenixd_rs::InvoiceRequest {
                    external_id: None,
                    description: None,
                    description_hash: Some(hex::encode(desc_hash.to_byte_array())),
                    amount_sat: amount_msats / 1000,
                    webhook_url: None,
                };

                let invoice_resp = client.create_invoice(invoice_req).await?;

                Ok(CreatedInvoice {
                    payment_request: invoice_resp.serialized,
                    payment_hash: invoice_resp.payment_hash,
                })
            }
            #[allow(unreachable_patterns)]
            _ => bail!("No node backend feature is enabled"),
        }
    }

    pub async fn lookup_invoice(
        &self,
        payment_hash: &str,
    ) -> anyhow::Result<Option<InvoiceStatus>> {
        match self {
            #[cfg(feature = "lnd")]
            Self::Lnd(lnd) => {
                let mut lnd = lnd.clone();
                let request = lnrpc::PaymentHash {
                    r_hash: hex::decode(payment_hash)?,
                    ..Default::default()
                };
                let resp = match lnd.lookup_invoice(request).await {
                    Ok(resp) => resp.into_inner(),
                    Err(_) => return Ok(None),
                };
                let settled = resp.state() == InvoiceState::Settled && !resp.r_preimage.is_empty();
                let preimage = (!resp.r_preimage.is_empty()).then(|| hex::encode(resp.r_preimage));
                Ok(Some(InvoiceStatus {
                    payment_request: Some(resp.payment_request),
                    settled,
                    preimage,
                }))
            }
            #[cfg(feature = "ldk-server")]
            Self::LdkServer(client) => {
                let resp = client
                    .get_payment_details(GetPaymentDetailsRequest {
                        payment_id: payment_hash.to_string(),
                    })
                    .await?;
                if let Some(payment) = resp.payment {
                    return Ok(Some(ldk_payment_status(payment)));
                }

                Ok(None)
            }
            #[cfg(feature = "phoenixd")]
            Self::Phoenixd(client) => {
                let invoice_resp = match client.get_incoming_invoice(payment_hash).await {
                    Ok(resp) => resp,
                    Err(_) => return Ok(None),
                };

                let settled = invoice_resp.is_paid;
                let preimage = if !invoice_resp.preimage.is_empty() {
                    Some(invoice_resp.preimage)
                } else {
                    None
                };

                Ok(Some(InvoiceStatus {
                    payment_request: invoice_resp.invoice,
                    settled,
                    preimage,
                }))
            }
            #[allow(unreachable_patterns)]
            _ => bail!("No node backend feature is enabled"),
        }
    }
}

#[cfg(feature = "ldk-server")]
fn ldk_payment_status(payment: Payment) -> InvoiceStatus {
    let settled = PaymentStatus::from_i32(payment.status) == Some(PaymentStatus::Succeeded);
    let preimage = payment
        .kind
        .and_then(|kind| kind.kind)
        .and_then(|kind| match kind {
            payment_kind::Kind::Bolt11(payment) => payment.preimage,
            payment_kind::Kind::Bolt11Jit(payment) => payment.preimage,
            _ => None,
        });

    InvoiceStatus {
        payment_request: None,
        settled,
        preimage,
    }
}

pub fn parse_invoice(invoice: &str) -> anyhow::Result<Bolt11Invoice> {
    Bolt11Invoice::from_str(invoice).map_err(|_| anyhow!("Invalid invoice format"))
}
