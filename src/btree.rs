//! Behavior tree module for building, validating, running, and deploying behavior trees.

use serde::{Deserialize, Serialize};

use crate::errors::IgrisError;
use crate::runtime::Runtime;

/// Result of validating a behavior tree definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BTreeValidateResult {
    pub valid: bool,
    pub root_type: Option<String>,
    pub root_name: Option<String>,
    pub error: Option<String>,
}

/// Result of executing a behavior tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BTreeRunResult {
    pub status: String,
    pub success: bool,
    pub tick_count: u64,
    pub duration_ms: u64,
    pub cancelled: bool,
    pub max_ticks_reached: bool,
    pub deadline_exceeded: bool,
    pub error: Option<String>,
}

/// Result of deploying a behavior tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BTreeDeployResult {
    pub deployed: bool,
    pub name: String,
    pub description: Option<String>,
}

/// Options for running a behavior tree.
pub struct BTreeRunOptions {
    pub context: Option<serde_json::Value>,
    pub max_ticks: u64,
    pub timeout_ms: u64,
}

impl Default for BTreeRunOptions {
    fn default() -> Self {
        Self {
            context: None,
            max_ticks: 1000,
            timeout_ms: 30000,
        }
    }
}

/// A behavior tree bound to a runtime, providing validate, run, and deploy operations.
pub struct BehaviorTree<'a> {
    tree: serde_json::Value,
    runtime: &'a Runtime,
}

impl<'a> BehaviorTree<'a> {
    /// Create a new BehaviorTree from a JSON definition and a runtime reference.
    pub fn new(tree: serde_json::Value, runtime: &'a Runtime) -> Self {
        Self { tree, runtime }
    }

    /// Create a BehaviorTree from a YAML string.
    ///
    /// Requires the `yaml` feature to be enabled:
    /// ```toml
    /// igris-inertial = { version = "0.1", features = ["yaml"] }
    /// ```
    #[cfg(feature = "yaml")]
    pub fn from_yaml(yaml_str: &str, runtime: &'a Runtime) -> Result<Self, IgrisError> {
        let tree: serde_json::Value = serde_yaml::from_str(yaml_str)
            .map_err(|e| IgrisError::Validation {
                message: format!("Failed to parse YAML: {}", e),
                status: 400,
                body: serde_json::json!({"error": e.to_string()}),
            })?;
        Ok(Self { tree, runtime })
    }

    /// Returns a reference to the underlying tree JSON.
    pub fn tree(&self) -> &serde_json::Value {
        &self.tree
    }

    /// Validate the behavior tree definition against the runtime.
    pub async fn validate(&self) -> Result<BTreeValidateResult, IgrisError> {
        let body = serde_json::json!({ "tree": self.tree });
        self.runtime
            .local_request(reqwest::Method::POST, "/v1/btree/validate", Some(&body))
            .await
    }

    /// Execute the behavior tree on the runtime with the given options.
    pub async fn run(&self, options: BTreeRunOptions) -> Result<BTreeRunResult, IgrisError> {
        let mut body = serde_json::json!({
            "tree": self.tree,
            "max_ticks": options.max_ticks,
            "timeout_ms": options.timeout_ms,
        });
        if let Some(ctx) = options.context {
            body["context"] = ctx;
        }
        self.runtime
            .local_request(reqwest::Method::POST, "/v1/btree/run", Some(&body))
            .await
    }

    /// Deploy the behavior tree as a named tree on the runtime.
    pub async fn deploy(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<BTreeDeployResult, IgrisError> {
        let mut body = serde_json::json!({
            "tree": self.tree,
            "name": name,
        });
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc.to_string());
        }
        self.runtime
            .local_request(reqwest::Method::POST, "/v1/btree/deploy", Some(&body))
            .await
    }
}

// ---------------------------------------------------------------------------
// Node builder helpers
// ---------------------------------------------------------------------------

/// Build a sequence composite node (succeeds only if all children succeed).
pub fn sequence_node(name: &str, children: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "type": "sequence",
        "name": name,
        "children": children,
    })
}

/// Build a selector (fallback) composite node (succeeds if any child succeeds).
pub fn selector_node(name: &str, children: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "type": "selector",
        "name": name,
        "children": children,
    })
}

/// Build an action leaf node that invokes a tool with arguments.
pub fn action_node(name: &str, tool: &str, args: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "action",
        "name": name,
        "tool": tool,
        "args": args,
    })
}

/// Build a condition leaf node that checks a context key against an expected value.
pub fn condition_node(name: &str, key: &str, expected: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "condition",
        "name": name,
        "key": key,
        "expected": expected,
    })
}

/// Build an LLM leaf node that invokes a language model with a prompt.
pub fn llm_node(name: &str, prompt: &str, model: Option<&str>) -> serde_json::Value {
    let mut node = serde_json::json!({
        "type": "llm_node",
        "name": name,
        "prompt": prompt,
    });
    if let Some(m) = model {
        node["model"] = serde_json::Value::String(m.to_string());
    }
    node
}

/// Alias for [`selector_node`] (tries children until one succeeds).
pub fn fallback_node(name: &str, children: Vec<serde_json::Value>) -> serde_json::Value {
    selector_node(name, children)
}
