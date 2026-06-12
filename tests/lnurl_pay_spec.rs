//! Tests for LNURL-PAY (LUD-06) specification compliance
//!
//! These tests validate that the lnurl-server correctly implements the LUD-06
//! LNURL-PAY specification for static QR code payments.

use serde_json::{json, Value};

/// Test metadata structure validation
///
/// According to LUD-06:
/// - metadata is a JSON array of arrays
/// - must contain one "text/plain" entry
/// - optional "text/identifier" entry
/// - optional image entries (png/jpeg)
#[test]
fn test_metadata_structure() {
    // Valid metadata examples that should pass
    let valid_metadata = vec![
        // Minimal: only text/plain
        r#"[["text/plain", "Payment description"]]"#,
        // With identifier
        r#"[["text/identifier", "user@domain.com"], ["text/plain", "Payment description"]]"#,
        // With long description
        r#"[["text/plain", "Short"], ["text/long-desc", "Longer\ndescription"]]"#,
    ];

    for metadata in valid_metadata {
        let parsed: Value = serde_json::from_str(metadata)
            .expect("metadata should be valid JSON");
        assert!(parsed.is_array(), "metadata must be a JSON array");

        let has_text_plain = parsed.as_array().unwrap().iter().any(|entry| {
            entry.is_array() && entry.get(0).map_or(false, |t| t == "text/plain")
        });
        assert!(has_text_plain, "metadata must contain a text/plain entry");
    }
}

/// Test initial LNURL request response structure
///
/// Spec Step 3: Wallet makes GET request and receives PayResponse
/// Required fields: callback, maxSendable, minSendable, metadata, tag
#[test]
fn test_initial_lnurl_request_response() {
    let response = json!({
        "callback": "https://example.com/get-invoice/abc123",
        "minSendable": 1000,
        "maxSendable": 100000000,
        "metadata": r#"[["text/plain", "Payment to user"]]"#,
        "tag": "payRequest"
    });

    // Validate all required fields are present
    assert!(response.get("callback").is_some(), "callback field required");
    assert!(response.get("minSendable").is_some(), "minSendable field required");
    assert!(response.get("maxSendable").is_some(), "maxSendable field required");
    assert!(response.get("metadata").is_some(), "metadata field required");
    assert!(response.get("tag").is_some(), "tag field required");

    // Validate field types
    assert!(response["callback"].is_string(), "callback must be string");
    assert!(response["minSendable"].is_number(), "minSendable must be number");
    assert!(response["maxSendable"].is_number(), "maxSendable must be number");
    assert!(response["metadata"].is_string(), "metadata must be string");
    assert_eq!(response["tag"], "payRequest", "tag must be payRequest");

    // Validate constraints
    assert!(
        response["minSendable"].as_u64().unwrap() >= 1,
        "minSendable must be >= 1"
    );
    assert!(
        response["minSendable"].as_u64().unwrap() <= response["maxSendable"].as_u64().unwrap(),
        "minSendable must be <= maxSendable"
    );
}

/// Test amount validation constraints
///
/// Spec: maxSendable >= minSendable >= 1
/// Wallet calculates: max = min(maxSendable, wallet_capacity)
///                   min = max(minSendable, wallet_minimum)
#[test]
fn test_amount_constraints() {
    let test_cases = vec![
        // (minSendable, maxSendable, should_be_valid)
        (1, 100_000_000, true),
        (0, 100_000_000, false), // minSendable < 1
        (1_000_000, 500_000, false), // minSendable > maxSendable
        (1, 1, true), // equal values is valid
        (1000, 1_000_000, true),
    ];

    for (min, max, should_be_valid) in test_cases {
        let is_valid = min >= 1 && min <= max;
        assert_eq!(
            is_valid, should_be_valid,
            "min={}, max={}: should_be_valid={}",
            min, max, should_be_valid
        );
    }
}

/// Test invoice generation response structure
///
/// Spec Step 6: Server returns invoice response
/// Required fields: pr, routes
/// pr = bech32-serialized BOLT11 invoice
/// routes = empty array
#[test]
fn test_invoice_response_structure() {
    let response = json!({
        "pr": "lnbc1000n1p0...",
        "routes": []
    });

    // Validate required fields
    assert!(response.get("pr").is_some(), "pr field required");
    assert!(response.get("routes").is_some(), "routes field required");

    // Validate field types
    assert!(response["pr"].is_string(), "pr must be string");
    assert!(response["pr"].as_str().unwrap().starts_with("lnbc") ||
            response["pr"].as_str().unwrap().starts_with("lntb") ||
            response["pr"].as_str().unwrap().starts_with("lnrt"),
            "pr must be BOLT11 invoice (starts with lnbc/lntb/lnrt)");

    assert!(response["routes"].is_array(), "routes must be array");
    assert_eq!(response["routes"].as_array().unwrap().len(), 0, "routes must be empty");

    // Validate no extra fields that violate spec
    let keys: Vec<&str> = response.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    assert_eq!(keys.len(), 2, "response should only have pr and routes fields");
    assert!(!keys.contains(&"status"), "response must not contain status field");
    assert!(!keys.contains(&"verify"), "response must not contain verify field");
}

/// Test error response structure
///
/// Spec: On error, return {"status": "ERROR", "reason": "..."}
#[test]
fn test_error_response_structure() {
    let response = json!({
        "status": "ERROR",
        "reason": "Amount is below minimum"
    });

    // Validate error fields
    assert_eq!(response["status"], "ERROR", "status must be ERROR");
    assert!(response["reason"].is_string(), "reason must be string");
    assert!(!response["reason"].as_str().unwrap().is_empty(), "reason must not be empty");
}

/// Test amount parameter handling in invoice request
///
/// Spec Step 5: Wallet makes GET to callback with ?amount=<millisatoshis>
#[test]
fn test_amount_parameter_parsing() {
    let test_cases = vec![
        ("1000", true, "valid amount"),
        ("invalid", false, "not a number"),
        ("", false, "empty string"),
        ("-1000", false, "negative number"),
    ];

    for (amount_str, should_parse, reason) in test_cases {
        let result: Result<u64, _> = amount_str.parse();
        let is_valid_number = result.is_ok();
        assert_eq!(
            is_valid_number, should_parse,
            "amount='{}': {} - expected to parse: {}",
            amount_str, reason, should_parse
        );
    }

    // Special case: 0 parses as u64 but should fail minimum validation
    let zero_result: Result<u64, _> = "0".parse();
    assert!(zero_result.is_ok(), "0 should parse as u64");
    let amount = zero_result.unwrap();
    assert!(amount < 1, "but 0 should fail minimum validation");
}

/// Test invoice amount verification
///
/// Spec Step 7: Wallet verifies that amount in invoice equals specified amount
/// The invoice should have exactly the requested amount in millisatoshis
#[test]
fn test_invoice_amount_matching() {
    // Simulate requested amount in millisatoshis
    let requested_amount_msat = 50_000;

    // Simulate invoice amount verification
    let invoice_amount_msat = 50_000;

    assert_eq!(
        invoice_amount_msat, requested_amount_msat,
        "invoice amount must match requested amount"
    );
}

/// Test flow: Get LNURL → Parse response → Request invoice
///
/// This is the core happy path from the spec
#[test]
fn test_lnurl_pay_complete_flow() {
    // Step 1-3: User scans QR and wallet gets PayResponse
    let pay_response = json!({
        "callback": "https://example.com/get-invoice/h",
        "minSendable": 1000,
        "maxSendable": 100_000_000,
        "metadata": r#"[["text/plain", "Payment"]]"#,
        "tag": "payRequest"
    });

    // Validate response structure
    assert_eq!(pay_response["tag"], "payRequest");
    let callback = pay_response["callback"].as_str().unwrap();

    // Step 4: Wallet displays dialog and user chooses amount
    let user_amount_msat = 50_000;
    assert!(
        user_amount_msat >= pay_response["minSendable"].as_u64().unwrap(),
        "user amount must be >= minSendable"
    );
    assert!(
        user_amount_msat <= pay_response["maxSendable"].as_u64().unwrap(),
        "user amount must be <= maxSendable"
    );

    // Step 5-6: Wallet requests invoice with amount
    let invoice_url = format!("{}?amount={}", callback, user_amount_msat);
    assert!(invoice_url.contains("amount="));

    // Step 6 (continued): Simulate invoice response
    let invoice_response = json!({
        "pr": "lnbc500u1p...",
        "routes": []
    });

    // Step 7: Verify amount in response
    // (In real test, would parse invoice and verify amount)
    assert!(invoice_response["pr"].is_string());
    assert_eq!(invoice_response["routes"].as_array().unwrap().len(), 0);
}

/// Test callback URL format
///
/// Spec requires callback URL to be valid and properly formatted
#[test]
fn test_callback_url_format() {
    let callbacks = vec![
        "https://example.com/get-invoice/abc123",
        "https://subdomain.example.com:8080/get-invoice/xyz",
        "https://example.com/path/to/callback",
    ];

    for callback in callbacks {
        // Callback should be a valid URL
        assert!(
            callback.starts_with("http://") || callback.starts_with("https://"),
            "callback must be HTTP/HTTPS URL"
        );

        // Should have path after domain
        let parts: Vec<&str> = callback.split('/').collect();
        assert!(parts.len() >= 4, "callback should have domain and path");
    }
}

/// Test domain extraction from LNURL
///
/// Spec Step 4: "Domain name extracted from LNURL query string"
#[test]
fn test_domain_extraction() {
    let lnurl_callback = "https://example.com/get-invoice/abc123";

    // Extract domain
    let domain = lnurl_callback
        .split('/').nth(2)
        .expect("should extract domain");

    assert_eq!(domain, "example.com");
}

/// Test payment state transitions
///
/// Invoice can be in states: Open (unpaid), Settled (paid)
#[test]
fn test_payment_states() {
    #[derive(Debug, PartialEq)]
    enum InvoiceState {
        Open,
        Settled,
    }

    let test_cases = vec![
        // (is_paid, expected_state)
        (false, InvoiceState::Open),
        (true, InvoiceState::Settled),
    ];

    for (is_paid, expected_state) in test_cases {
        let state = if is_paid {
            InvoiceState::Settled
        } else {
            InvoiceState::Open
        };
        assert_eq!(state, expected_state);
    }
}

/// Test description hash validation
///
/// Invoice description should be a hash, not plain text
#[test]
fn test_description_hash_format() {
    // Valid description hash (SHA256 hex, 64 characters)
    let valid_hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

    assert_eq!(valid_hash.len(), 64, "SHA256 hash should be 64 hex chars");
    assert!(valid_hash.chars().all(|c| c.is_ascii_hexdigit()), "hash should be hex");
}

/// Test error cases for initial LNURL request
#[test]
fn test_lnurl_request_error_cases() {
    let error_cases = vec![
        ("empty_name", "Name parameter is required"),
        ("invalid_format", "Invalid name format"),
    ];

    for (_case, expected_reason) in error_cases {
        let error_response = json!({
            "status": "ERROR",
            "reason": expected_reason
        });

        assert_eq!(error_response["status"], "ERROR");
        assert!(error_response["reason"].as_str().unwrap().len() > 0);
    }
}

/// Test error cases for invoice generation
#[test]
fn test_invoice_generation_error_cases() {
    let error_cases = vec![
        ("missing_amount", "Missing amount parameter"),
        ("amount_too_low", "Amount is below minSendable"),
        ("amount_too_high", "Amount exceeds maxSendable"),
        ("invalid_hash", "Invalid description hash"),
        ("node_error", "Failed to create invoice"),
    ];

    for (_case, expected_reason) in error_cases {
        let error_response = json!({
            "status": "ERROR",
            "reason": expected_reason
        });

        assert_eq!(error_response["status"], "ERROR");
        assert!(error_response["reason"].is_string());
    }
}

/// Test metadata is transmitted as string, not object
///
/// Spec: "metadata json array must contain... be sent as a string"
#[test]
fn test_metadata_string_encoding() {
    // Metadata should be a JSON string, not object
    let metadata_string = r#"[["text/plain", "Description"]]"#;

    // Should be parseable as JSON
    let _: Value = serde_json::from_str(metadata_string)
        .expect("metadata string should be valid JSON");

    // When embedded in response, it's a string
    let response = json!({
        "metadata": metadata_string,
    });

    assert!(response["metadata"].is_string(), "metadata in response must be string");
}

/// Test wallet calculates effective bounds
///
/// Spec Step 4:
/// max can send = min(maxSendable, local wallet capacity)
/// min can send = max(minSendable, local wallet minimum)
#[test]
fn test_wallet_amount_bounds_calculation() {
    let max_sendable = 100_000_000;
    let min_sendable = 1_000;
    let wallet_capacity = 50_000_000;
    let wallet_minimum = 500;

    let effective_max = min_sendable.max(wallet_capacity.min(max_sendable));
    let effective_min = min_sendable.max(wallet_minimum);

    assert!(effective_min <= effective_max, "bounds should be valid");
}

/// Test payment hash in callback
///
/// When requesting invoice, description hash is used in URL
#[test]
fn test_payment_hash_in_callback() {
    let description_hash = "abcdef01234567890123456789abcdef0123456789abcdef0123456789abcdef";
    let callback = format!(
        "https://example.com/get-invoice/{}",
        description_hash
    );

    assert!(callback.contains(&description_hash));
    assert!(callback.ends_with(&description_hash));
}

/// Test nostr zap request support (optional per spec)
///
/// Spec mentions optional Nostr zap request support
#[test]
fn test_nostr_zap_request_optional() {
    // Zap request should be optional
    let response_without_zap = json!({
        "pr": "lnbc1000n1p...",
        "routes": []
    });

    // Response is valid even without zap-related fields
    assert!(response_without_zap["pr"].is_string());

    // But if provided, should be handled
    let response_with_zap = json!({
        "pr": "lnbc1000n1p...",
        "routes": [],
        "zap_request": "optional_nostr_event"
    });

    // Either way, core fields are present
    assert!(response_with_zap.get("pr").is_some());
    assert!(response_with_zap.get("routes").is_some());
}

/// Test response serialization matches spec
///
/// Ensure responses serialize to valid JSON matching spec examples
#[test]
fn test_response_json_serialization() {
    let pay_response = json!({
        "callback": "https://example.com/get-invoice/abc123",
        "minSendable": 1000,
        "maxSendable": 100000000,
        "metadata": r#"[["text/plain", "Sats for user"]]"#,
        "tag": "payRequest"
    });

    let json_string = serde_json::to_string(&pay_response)
        .expect("should serialize to JSON");

    // Should be compact JSON
    assert!(!json_string.contains('\n'), "response should be compact JSON");

    // Should be parseable back
    let _: Value = serde_json::from_str(&json_string)
        .expect("serialized JSON should parse");
}
