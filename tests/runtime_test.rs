use igris_inertial::{InferRequest, Message, Runtime, RuntimeBuilder};

#[test]
fn test_runtime_builder_construction() {
    let runtime = RuntimeBuilder::new("http://localhost:9090")
        .cloud_url("https://cloud.igris.dev")
        .auto_fallback(true)
        .timeout(std::time::Duration::from_secs(15))
        .local_model("llama-7b")
        .build();

    assert!(runtime.is_ok());
    let rt = runtime.unwrap();
    assert_eq!(rt.config().local_url, "http://localhost:9090");
    assert_eq!(
        rt.config().cloud_url.as_deref(),
        Some("https://cloud.igris.dev")
    );
    assert!(rt.config().auto_fallback);
    assert_eq!(rt.config().timeout, std::time::Duration::from_secs(15));
    assert_eq!(rt.config().local_model.as_deref(), Some("llama-7b"));
}

#[test]
fn test_runtime_new_defaults() {
    let runtime = Runtime::new("http://localhost:8080");
    assert!(runtime.is_ok());
    let rt = runtime.unwrap();
    assert_eq!(rt.config().local_url, "http://localhost:8080");
    assert!(rt.config().cloud_url.is_none());
    assert!(rt.config().auto_fallback);
    assert_eq!(rt.config().timeout, std::time::Duration::from_secs(30));
    assert!(rt.config().local_model.is_none());
}

#[test]
fn test_runtime_builder_via_runtime() {
    let runtime = Runtime::builder("http://localhost:3000")
        .auto_fallback(false)
        .build();

    assert!(runtime.is_ok());
    let rt = runtime.unwrap();
    assert!(!rt.config().auto_fallback);
}

fn sample_infer_request() -> InferRequest {
    InferRequest {
        model: "llama-7b".to_string(),
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
    }
}

fn chat_response_json() -> serde_json::Value {
    serde_json::json!({
        "id": "rt-123",
        "object": "chat.completion",
        "created": 1700000000_i64,
        "model": "llama-7b",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hi there!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 5,
            "completion_tokens": 3,
            "total_tokens": 8
        }
    })
}

#[tokio::test]
async fn test_chat_local_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(chat_response_json().to_string())
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let request = sample_infer_request();
    let result = runtime.chat(&request).await;

    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.id, "rt-123");
    assert_eq!(resp.model, "llama-7b");
    assert_eq!(resp.choices.len(), 1);
    assert_eq!(resp.choices[0].message.content, "Hi there!");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_local_only_no_fallback() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(chat_response_json().to_string())
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let request = sample_infer_request();
    let result = runtime.chat_local(&request).await;

    assert!(result.is_ok());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_fallback_to_cloud() {
    // Local server returns connection error (unreachable port).
    // Cloud server is a mock that succeeds.
    let mut cloud_server = mockito::Server::new_async().await;
    let cloud_mock = cloud_server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(chat_response_json().to_string())
        .create_async()
        .await;

    let runtime = Runtime::builder("http://127.0.0.1:1") // unreachable
        .cloud_url(cloud_server.url())
        .auto_fallback(true)
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();

    let request = sample_infer_request();
    let result = runtime.chat(&request).await;

    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.id, "rt-123");
    cloud_mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_no_fallback_when_disabled() {
    let runtime = Runtime::builder("http://127.0.0.1:1") // unreachable
        .cloud_url("http://127.0.0.1:2")
        .auto_fallback(false)
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();

    let request = sample_infer_request();
    let result = runtime.chat(&request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_model() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/admin/models/load")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"loaded","model_id":"my-model"}"#)
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let result = runtime
        .load_model("/models/llama-7b.gguf", Some("my-model"))
        .await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["status"], "loaded");
    assert_eq!(val["model_id"], "my-model");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_swap_model() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/admin/models/swap")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"swapped","model_id":"llama-13b"}"#)
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let result = runtime.swap_model("llama-13b").await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["status"], "swapped");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_list_models() {
    let mut server = mockito::Server::new_async().await;
    let body = serde_json::json!([
        {"id": "llama-7b", "status": "loaded"},
        {"id": "mistral-7b", "status": "available"}
    ]);
    let mock = server
        .mock("GET", "/v1/admin/models")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body.to_string())
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let result = runtime.list_models().await;

    assert!(result.is_ok());
    let models = result.unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0]["id"], "llama-7b");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_health() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/v1/health")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"ok","version":"0.1.0","uptime":12345.6}"#)
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let result = runtime.health().await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["status"], "ok");
    assert_eq!(val["version"], "0.1.0");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_api_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"internal server error"}"#)
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let request = sample_infer_request();
    let result = runtime.chat(&request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, igris_inertial::IgrisError::Api { status_code: 500, .. }),
        "expected Api error with status 500, got: {:?}",
        err
    );
    mock.assert_async().await;
}
