use igris_inertial::{
    action_node, condition_node, selector_node, sequence_node, BTreeRunOptions, BehaviorTree,
    Runtime,
};

// ---------------------------------------------------------------------------
// Validate
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_validate_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/validate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "valid": true,
                "root_type": "sequence",
                "root_name": "main",
                "error": null
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let tree = sequence_node("main", vec![action_node("greet", "say_hello", serde_json::json!({}))]);
    let bt = BehaviorTree::new(tree, &runtime);
    let result = bt.validate().await;

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.valid);
    assert_eq!(res.root_type.as_deref(), Some("sequence"));
    assert_eq!(res.root_name.as_deref(), Some("main"));
    assert!(res.error.is_none());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_validate_invalid_tree() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/validate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "valid": false,
                "root_type": null,
                "root_name": null,
                "error": "missing root node"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let bt = BehaviorTree::new(serde_json::json!({}), &runtime);
    let result = bt.validate().await;

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(!res.valid);
    assert_eq!(res.error.as_deref(), Some("missing root node"));
    mock.assert_async().await;
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/run")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "status": "success",
                "success": true,
                "tick_count": 3,
                "duration_ms": 42,
                "cancelled": false,
                "max_ticks_reached": false,
                "deadline_exceeded": false,
                "error": null
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let tree = sequence_node("root", vec![]);
    let bt = BehaviorTree::new(tree, &runtime);
    let result = bt.run(BTreeRunOptions::default()).await;

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.success);
    assert_eq!(res.status, "success");
    assert_eq!(res.tick_count, 3);
    assert_eq!(res.duration_ms, 42);
    assert!(!res.cancelled);
    assert!(!res.max_ticks_reached);
    assert!(!res.deadline_exceeded);
    assert!(res.error.is_none());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_run_with_options() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/run")
        .match_body(mockito::Matcher::PartialJsonString(
            serde_json::json!({
                "max_ticks": 50,
                "timeout_ms": 5000,
                "context": { "env": "test" }
            })
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "status": "success",
                "success": true,
                "tick_count": 1,
                "duration_ms": 10,
                "cancelled": false,
                "max_ticks_reached": false,
                "deadline_exceeded": false,
                "error": null
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let tree = sequence_node("root", vec![]);
    let bt = BehaviorTree::new(tree, &runtime);
    let options = BTreeRunOptions {
        context: Some(serde_json::json!({ "env": "test" })),
        max_ticks: 50,
        timeout_ms: 5000,
    };
    let result = bt.run(options).await;

    assert!(result.is_ok());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_run_max_ticks_reached() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/run")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "status": "failure",
                "success": false,
                "tick_count": 1000,
                "duration_ms": 500,
                "cancelled": false,
                "max_ticks_reached": true,
                "deadline_exceeded": false,
                "error": "max ticks reached"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let tree = sequence_node("root", vec![]);
    let bt = BehaviorTree::new(tree, &runtime);
    let result = bt.run(BTreeRunOptions::default()).await;

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(!res.success);
    assert!(res.max_ticks_reached);
    assert_eq!(res.error.as_deref(), Some("max ticks reached"));
    mock.assert_async().await;
}

// ---------------------------------------------------------------------------
// Deploy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_deploy_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/deploy")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "deployed": true,
                "name": "patrol-bot",
                "description": "Patrol behavior"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let tree = sequence_node("patrol", vec![]);
    let bt = BehaviorTree::new(tree, &runtime);
    let result = bt.deploy("patrol-bot", Some("Patrol behavior")).await;

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.deployed);
    assert_eq!(res.name, "patrol-bot");
    assert_eq!(res.description.as_deref(), Some("Patrol behavior"));
    mock.assert_async().await;
}

#[tokio::test]
async fn test_deploy_without_description() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/deploy")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "deployed": true,
                "name": "simple-tree",
                "description": null
            })
            .to_string(),
        )
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let tree = sequence_node("root", vec![]);
    let bt = BehaviorTree::new(tree, &runtime);
    let result = bt.deploy("simple-tree", None).await;

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.deployed);
    assert!(res.description.is_none());
    mock.assert_async().await;
}

// ---------------------------------------------------------------------------
// Node builders
// ---------------------------------------------------------------------------

#[test]
fn test_sequence_node() {
    let node = sequence_node("seq", vec![]);
    assert_eq!(node["type"], "sequence");
    assert_eq!(node["name"], "seq");
    assert!(node["children"].as_array().unwrap().is_empty());
}

#[test]
fn test_selector_node() {
    let node = selector_node("sel", vec![]);
    assert_eq!(node["type"], "selector");
    assert_eq!(node["name"], "sel");
    assert!(node["children"].as_array().unwrap().is_empty());
}

#[test]
fn test_action_node() {
    let node = action_node("do-thing", "my_tool", serde_json::json!({"key": "val"}));
    assert_eq!(node["type"], "action");
    assert_eq!(node["name"], "do-thing");
    assert_eq!(node["tool"], "my_tool");
    assert_eq!(node["args"]["key"], "val");
}

#[test]
fn test_condition_node() {
    let node = condition_node("check-env", "environment", "production");
    assert_eq!(node["type"], "condition");
    assert_eq!(node["name"], "check-env");
    assert_eq!(node["key"], "environment");
    assert_eq!(node["expected"], "production");
}

#[test]
fn test_nested_tree_construction() {
    let tree = selector_node(
        "root",
        vec![
            sequence_node(
                "happy-path",
                vec![
                    condition_node("is-ready", "status", "ready"),
                    action_node("execute", "run_task", serde_json::json!({"retries": 3})),
                ],
            ),
            action_node("fallback", "log_error", serde_json::json!({})),
        ],
    );

    assert_eq!(tree["type"], "selector");
    let children = tree["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0]["type"], "sequence");
    assert_eq!(children[0]["children"].as_array().unwrap().len(), 2);
    assert_eq!(children[1]["type"], "action");
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_validate_server_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/validate")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"internal failure"}"#)
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let bt = BehaviorTree::new(serde_json::json!({}), &runtime);
    let result = bt.validate().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, igris_inertial::IgrisError::Api { status_code: 500, .. }),
        "expected Api error with status 500, got: {:?}",
        err
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn test_run_validation_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/btree/run")
        .with_status(422)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"invalid tree structure"}"#)
        .create_async()
        .await;

    let runtime = Runtime::new(server.url()).unwrap();
    let bt = BehaviorTree::new(serde_json::json!({}), &runtime);
    let result = bt.run(BTreeRunOptions::default()).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, igris_inertial::IgrisError::Validation { status_code: 422, .. }),
        "expected Validation error with status 422, got: {:?}",
        err
    );
    mock.assert_async().await;
}
