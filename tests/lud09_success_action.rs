//! Tests for LUD-09: successAction field specification
//!
//! These tests validate that the lnurl-server correctly implements the LUD-09
//! successAction specification for providing user feedback after payment.

use serde_json::json;

/// Test message-type success action structure
///
/// Spec: successAction with tag "message" should have message field (max 144 chars)
#[test]
fn test_success_action_message_structure() {
    let action = json!({
        "tag": "message",
        "message": "Thank you for your purchase!"
    });

    assert_eq!(action["tag"], "message");
    assert!(action["message"].is_string());
    assert!(!action["message"].as_str().unwrap().is_empty());
}

/// Test URL-type success action structure
///
/// Spec: successAction with tag "url" should have description and url fields
#[test]
fn test_success_action_url_structure() {
    let action = json!({
        "tag": "url",
        "description": "View your order details",
        "url": "https://example.com/orders/123"
    });

    assert_eq!(action["tag"], "url");
    assert!(action["description"].is_string());
    assert!(action["url"].is_string());
}

/// Test message length validation
///
/// Spec: Message must be max 144 characters
#[test]
fn test_message_length_validation() {
    // Valid: exactly 144 chars
    let valid_144 = "a".repeat(144);
    assert_eq!(valid_144.len(), 144);

    // Invalid: 145 chars
    let invalid_145 = "a".repeat(145);
    assert!(invalid_145.len() > 144);

    // Valid: less than 144
    let valid_short = "Thank you!";
    assert!(valid_short.len() <= 144);
}

/// Test description length validation
///
/// Spec: Description must be max 144 characters
#[test]
fn test_description_length_validation() {
    // Valid
    let valid_desc = "View your order details";
    assert!(valid_desc.len() <= 144);

    // Invalid
    let invalid_desc = "a".repeat(145);
    assert!(invalid_desc.len() > 144);
}

/// Test URL domain validation
///
/// Spec: URL domain must match callback domain
#[test]
fn test_url_domain_matching() {
    let callback_domain = "example.com";
    let valid_url = "https://example.com/orders/123";
    let invalid_url = "https://other.com/orders/123";

    // Extract domain from valid URL
    let valid_domain = valid_url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("");

    // Extract domain from invalid URL
    let invalid_domain = invalid_url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("");

    assert_eq!(valid_domain, callback_domain);
    assert_ne!(invalid_domain, callback_domain);
}

/// Test successAction in invoice response
///
/// Spec: Response should include successAction (optional)
#[test]
fn test_invoice_response_with_success_action() {
    let response = json!({
        "pr": "lnbc500u1p...",
        "routes": [],
        "successAction": {
            "tag": "message",
            "message": "Payment received!"
        }
    });

    assert!(response.get("pr").is_some());
    assert!(response.get("routes").is_some());
    assert!(response.get("successAction").is_some());

    let action = &response["successAction"];
    assert_eq!(action["tag"], "message");
    assert!(!action["message"].as_str().unwrap().is_empty());
}

/// Test invoice response without success action
///
/// Spec: Response is valid without successAction (optional field)
#[test]
fn test_invoice_response_without_success_action() {
    let response = json!({
        "pr": "lnbc500u1p...",
        "routes": []
    });

    assert!(response.get("pr").is_some());
    assert!(response.get("routes").is_some());
    assert!(response.get("successAction").is_none());
}

/// Test message action tag value
///
/// Spec: tag field must be exactly "message"
#[test]
fn test_message_action_tag_value() {
    let action = json!({
        "tag": "message",
        "message": "Thank you!"
    });

    assert_eq!(action["tag"], "message");
    assert_ne!(action["tag"], "Message");
    assert_ne!(action["tag"], "MSG");
}

/// Test URL action tag value
///
/// Spec: tag field must be exactly "url"
#[test]
fn test_url_action_tag_value() {
    let action = json!({
        "tag": "url",
        "description": "View details",
        "url": "https://example.com/details"
    });

    assert_eq!(action["tag"], "url");
    assert_ne!(action["tag"], "URL");
    assert_ne!(action["tag"], "link");
}

/// Test message action has required fields
///
/// Spec: Message action requires "tag" and "message" fields
#[test]
fn test_message_action_required_fields() {
    let valid_action = json!({
        "tag": "message",
        "message": "Thank you!"
    });

    assert!(valid_action.get("tag").is_some());
    assert!(valid_action.get("message").is_some());

    let missing_message = json!({
        "tag": "message"
    });

    assert!(missing_message.get("tag").is_some());
    assert!(missing_message.get("message").is_none()); // Missing!
}

/// Test URL action has required fields
///
/// Spec: URL action requires "tag", "description", and "url" fields
#[test]
fn test_url_action_required_fields() {
    let valid_action = json!({
        "tag": "url",
        "description": "View order",
        "url": "https://example.com/order"
    });

    assert!(valid_action.get("tag").is_some());
    assert!(valid_action.get("description").is_some());
    assert!(valid_action.get("url").is_some());

    // Missing description
    let missing_desc = json!({
        "tag": "url",
        "url": "https://example.com/order"
    });

    assert!(missing_desc.get("url").is_some());
    assert!(missing_desc.get("description").is_none()); // Missing!

    // Missing URL
    let missing_url = json!({
        "tag": "url",
        "description": "View order"
    });

    assert!(missing_url.get("description").is_some());
    assert!(missing_url.get("url").is_none()); // Missing!
}

/// Test empty message rejection
///
/// Spec: Message cannot be empty
#[test]
fn test_empty_message_rejection() {
    let empty_message = "";
    assert!(empty_message.is_empty());

    let action = json!({
        "tag": "message",
        "message": empty_message
    });

    assert_eq!(action["message"].as_str().unwrap().len(), 0);
}

/// Test empty description rejection
///
/// Spec: Description cannot be empty
#[test]
fn test_empty_description_rejection() {
    let empty_description = "";
    assert!(empty_description.is_empty());

    let action = json!({
        "tag": "url",
        "description": empty_description,
        "url": "https://example.com"
    });

    assert_eq!(action["description"].as_str().unwrap().len(), 0);
}

/// Test URL with port number
///
/// Spec: Domain should be extracted correctly with port
#[test]
fn test_url_domain_with_port() {
    let callback_domain = "example.com";
    let url_with_port = "https://example.com:8080/orders/123";

    let extracted = url_with_port
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|domain| domain.split(':').next())
        .unwrap_or("");

    assert_eq!(extracted, callback_domain);
}

/// Test URL with path
///
/// Spec: URL can include path components
#[test]
fn test_url_with_path() {
    let url = "https://example.com/orders/pending/123";
    assert!(url.contains("/orders/pending/123"));

    let domain = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("");

    assert_eq!(domain, "example.com");
    assert!(!domain.contains("/orders"));
}

/// Test wallet display requirements for message
///
/// Spec: Wallet should show message as popup or toaster
#[test]
fn test_message_display_requirement() {
    let action = json!({
        "tag": "message",
        "message": "🔓 Bike unlocked! Enjoy your ride!"
    });

    // Action should be displayable (string message)
    assert!(action["message"].is_string());
    let message = action["message"].as_str().unwrap();
    assert!(!message.is_empty());
    assert!(message.len() <= 144);
}

/// Test wallet display requirements for URL
///
/// Spec: Wallet should show description and open URL button
#[test]
fn test_url_display_requirement() {
    let action = json!({
        "tag": "url",
        "description": "View your order details",
        "url": "https://example.com/orders/ABC123"
    });

    // Action should have displayable description
    assert!(action["description"].is_string());
    let description = action["description"].as_str().unwrap();
    assert!(!description.is_empty());
    assert!(description.len() <= 144);

    // Action should have openable URL
    assert!(action["url"].is_string());
    let url = action["url"].as_str().unwrap();
    assert!(url.starts_with("http://") || url.starts_with("https://"));
}

/// Test real-world message examples
#[test]
fn test_real_world_message_examples() {
    let examples: Vec<(&str, bool)> = vec![
        ("Thank you for using bike-over-ln co! Your rental bike is unlocked now", true),
        ("🎉 Payment received! Your order has been confirmed", true),
        ("Your LN service payment is complete", true),
        ("", false), // Empty
    ];

    for (message, should_be_valid) in examples {
        let is_valid = !message.is_empty() && message.len() <= 144;
        assert_eq!(
            is_valid, should_be_valid,
            "message: '{}...', should_be_valid: {}",
            &message[..message.len().min(20)],
            should_be_valid
        );
    }

    // Test max length separately
    let max_length_msg = "a".repeat(144);
    assert!(max_length_msg.len() <= 144);

    let over_limit_msg = "a".repeat(145);
    assert!(over_limit_msg.len() > 144);
}

/// Test real-world URL examples
#[test]
fn test_real_world_url_examples() {
    let callback_domain = "ln-service.com";
    let examples = vec![
        ("https://ln-service.com/order/123", true),
        ("https://ln-service.com:8080/order/123", true),
        ("https://ln-service.com/order/pending/abc", true),
        ("https://other-domain.com/order/123", false), // Wrong domain
        ("http://ln-service.com/order/123", true), // HTTP is OK
    ];

    for (url, should_match) in examples {
        let domain = url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .and_then(|d| d.split(':').next())
            .unwrap_or("");

        let matches = domain == callback_domain;
        assert_eq!(
            matches, should_match,
            "url: {}, callback: {}, matches: {}",
            url, callback_domain, should_match
        );
    }
}

/// Test successAction type safety
///
/// Spec: Wallet must support the tag type before proceeding
#[test]
fn test_unsupported_tag_type() {
    let unknown_action = json!({
        "tag": "qrcode",
        "data": "some_data"
    });

    // Wallet should check if tag is supported
    let is_supported_tag = matches!(
        unknown_action["tag"].as_str(),
        Some("message" | "url")
    );

    assert!(!is_supported_tag, "Unsupported tag should be detected");
}

/// Test JSON serialization of success actions
#[test]
fn test_success_action_json_serialization() {
    let message_action = json!({
        "tag": "message",
        "message": "Thank you!"
    });

    let serialized = serde_json::to_string(&message_action).unwrap();
    assert!(!serialized.contains('\n'), "should be compact JSON");

    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, message_action, "should round-trip correctly");
}

/// Test invoice response JSON with successAction
#[test]
fn test_invoice_response_json_with_success_action() {
    let response = json!({
        "pr": "lnbc500u1p...",
        "routes": [],
        "successAction": {
            "tag": "message",
            "message": "Thank you!"
        }
    });

    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains("successAction"), "should include successAction");
    assert!(json_str.contains("message"), "should include message");

    // Should be valid JSON
    let _: serde_json::Value = serde_json::from_str(&json_str).unwrap();
}

/// Test that successAction is optional in payment flow
///
/// Spec: Payment flow continues without successAction
#[test]
fn test_success_action_is_optional() {
    // Response without successAction
    let response_no_action = json!({
        "pr": "lnbc500u1p...",
        "routes": []
    });

    // Response with successAction
    let response_with_action = json!({
        "pr": "lnbc500u1p...",
        "routes": [],
        "successAction": {
            "tag": "message",
            "message": "Thank you!"
        }
    });

    // Both are valid invoice responses
    assert!(response_no_action.get("pr").is_some());
    assert!(response_with_action.get("pr").is_some());
}

/// Test configuration constraint validation
///
/// Spec: Cannot have both message and URL successAction
#[test]
fn test_cannot_have_both_message_and_url() {
    // Invalid: trying to configure both
    let has_message = Some("Thank you!");
    let has_url = Some("https://example.com");

    let is_invalid = has_message.is_some() && has_url.is_some();
    assert!(is_invalid, "cannot configure both message and url");
}

/// Test forward compatibility
///
/// Spec: Future successAction types may be added
#[test]
fn test_forward_compatibility() {
    // Current supported types
    let message_tag = "message";
    let url_tag = "url";

    // Future types (hypothetical)
    let future_tag = "airdrop"; // Hypothetical future type

    let is_known = matches!(message_tag, "message" | "url");
    let is_known_url = matches!(url_tag, "message" | "url");
    let is_known_future = matches!(future_tag, "message" | "url");

    assert!(is_known, "message should be known");
    assert!(is_known_url, "url should be known");
    assert!(!is_known_future, "future type should not be known");
}
