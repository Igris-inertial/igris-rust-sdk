use igris_inertial::{IgrisClient, InferRequest, Message};

#[test]
fn test_client_builder() {
    let client = IgrisClient::builder("http://localhost:8080")
        .api_key("test-key")
        .timeout(std::time::Duration::from_secs(10))
        .build();

    assert!(client.is_ok());
}

#[test]
fn test_client_new() {
    let client = IgrisClient::new("http://localhost:8080", "test-key");
    assert!(client.is_ok());
}

#[test]
fn test_infer_request_serialization() {
    let req = InferRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
            content_parts: None,
        }],
        stream: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        policy: None,
        metadata: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("gpt-4"));
    assert!(json.contains("Hello"));
    assert!(!json.contains("stream")); // None fields skipped
}
