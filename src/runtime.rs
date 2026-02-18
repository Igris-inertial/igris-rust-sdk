//! Runtime module for local/cloud inference with automatic fallback.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use serde::Serialize;

use crate::containment::{Bounds, ViolationRecord};
use crate::errors::IgrisError;
use crate::types::{InferRequest, InferResponse};

/// Configuration for the Runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub local_url: String,
    pub cloud_url: Option<String>,
    pub auto_fallback: bool,
    pub timeout: std::time::Duration,
    pub local_model: Option<String>,
    pub bounds: Option<Bounds>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            local_url: "http://localhost:8080".to_string(),
            cloud_url: None,
            auto_fallback: true,
            timeout: std::time::Duration::from_secs(30),
            local_model: None,
            bounds: None,
        }
    }
}

/// Builder for configuring a Runtime.
pub struct RuntimeBuilder {
    config: RuntimeConfig,
}

impl RuntimeBuilder {
    pub fn new(local_url: impl Into<String>) -> Self {
        Self {
            config: RuntimeConfig {
                local_url: local_url.into(),
                ..RuntimeConfig::default()
            },
        }
    }

    pub fn cloud_url(mut self, url: impl Into<String>) -> Self {
        self.config.cloud_url = Some(url.into());
        self
    }

    pub fn auto_fallback(mut self, enabled: bool) -> Self {
        self.config.auto_fallback = enabled;
        self
    }

    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn local_model(mut self, model: impl Into<String>) -> Self {
        self.config.local_model = Some(model.into());
        self
    }

    /// Set containment bounds forwarded via `X-Igris-Bounds` on every request.
    pub fn bounds(mut self, bounds: Bounds) -> Self {
        self.config.bounds = Some(bounds);
        self
    }

    pub fn build(self) -> Result<Runtime, IgrisError> {
        let local_http = Self::build_http_client(self.config.timeout)?;
        let cloud_http = if self.config.cloud_url.is_some() {
            Some(Self::build_http_client(self.config.timeout)?)
        } else {
            None
        };

        Ok(Runtime {
            config: self.config,
            local_http,
            cloud_http,
        })
    }

    fn build_http_client(timeout: std::time::Duration) -> Result<Client, IgrisError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http = Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .build()?;

        Ok(http)
    }
}

#[derive(Debug, serde::Deserialize)]
struct ViolationsResponse {
    #[serde(default)]
    violations: Vec<ViolationRecord>,
}

/// Runtime client for local inference with optional cloud fallback.
pub struct Runtime {
    config: RuntimeConfig,
    local_http: Client,
    cloud_http: Option<Client>,
}

impl Runtime {
    /// Create a new runtime with builder pattern.
    pub fn builder(local_url: impl Into<String>) -> RuntimeBuilder {
        RuntimeBuilder::new(local_url)
    }

    /// Simple constructor with defaults.
    pub fn new(local_url: impl Into<String>) -> Result<Self, IgrisError> {
        Self::builder(local_url).build()
    }

    /// Returns a reference to the current runtime configuration.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    fn local_url(&self, path: &str) -> String {
        format!("{}{}", self.config.local_url.trim_end_matches('/'), path)
    }

    fn cloud_url(&self, path: &str) -> Option<String> {
        self.config
            .cloud_url
            .as_ref()
            .map(|base| format!("{}{}", base.trim_end_matches('/'), path))
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<T, IgrisError> {
        let status = resp.status().as_u16();

        if status == 401 || status == 403 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Authentication {
                message: text,
                status_code: status,
            });
        }
        if status == 429 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::RateLimit { message: text });
        }
        if status == 400 || status == 422 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Validation {
                message: text,
                status_code: status,
            });
        }
        if status >= 400 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Api {
                message: text,
                status_code: status,
            });
        }

        let data = resp.json().await?;
        Ok(data)
    }

    pub(crate) async fn local_request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T, IgrisError> {
        let url = self.local_url(path);
        let mut req = self.local_http.request(method, &url);
        if let Some(b) = body {
            req = req.json(b);
        }
        if let Some(bounds) = &self.config.bounds {
            if let Ok(hv) = HeaderValue::from_str(&bounds.to_header_value()) {
                req = req.header(
                    HeaderName::from_static("x-igris-bounds"),
                    hv,
                );
            }
        }
        let resp = req.send().await?;
        Self::handle_response(resp).await
    }

    async fn cloud_request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T, IgrisError> {
        let cloud_http = self
            .cloud_http
            .as_ref()
            .ok_or_else(|| IgrisError::Api {
                message: "no cloud URL configured".to_string(),
                status_code: 0,
            })?;
        let url = self.cloud_url(path).ok_or_else(|| IgrisError::Api {
            message: "no cloud URL configured".to_string(),
            status_code: 0,
        })?;

        let mut req = cloud_http.request(method, &url);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        Self::handle_response(resp).await
    }

    async fn request_with_fallback<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T, IgrisError> {
        match self.local_request(method.clone(), path, body).await {
            Ok(result) => Ok(result),
            Err(IgrisError::Network(_))
                if self.config.auto_fallback && self.config.cloud_url.is_some() =>
            {
                self.cloud_request(method, path, body).await
            }
            Err(e) => Err(e),
        }
    }

    /// Send a chat completion request, falling back to cloud if configured.
    pub async fn chat(&self, request: &InferRequest) -> Result<InferResponse, IgrisError> {
        self.request_with_fallback(reqwest::Method::POST, "/v1/chat/completions", Some(request))
            .await
    }

    /// Send a chat completion request to the local runtime only (no fallback).
    pub async fn chat_local(&self, request: &InferRequest) -> Result<InferResponse, IgrisError> {
        self.local_request(reqwest::Method::POST, "/v1/chat/completions", Some(request))
            .await
    }

    /// Load a GGUF model into the local runtime.
    pub async fn load_model(
        &self,
        model_path: &str,
        model_id: Option<&str>,
    ) -> Result<serde_json::Value, IgrisError> {
        let mut body = serde_json::json!({ "model_path": model_path });
        if let Some(id) = model_id {
            body["model_id"] = serde_json::Value::String(id.to_string());
        }
        self.local_request(reqwest::Method::POST, "/v1/admin/models/load", Some(&body))
            .await
    }

    /// Hot-swap to a different model on the local runtime.
    pub async fn swap_model(&self, model_id: &str) -> Result<serde_json::Value, IgrisError> {
        let body = serde_json::json!({ "model_id": model_id });
        self.local_request(reqwest::Method::POST, "/v1/admin/models/swap", Some(&body))
            .await
    }

    /// List available models on the local runtime.
    pub async fn list_models(&self) -> Result<Vec<serde_json::Value>, IgrisError> {
        self.local_request::<Vec<serde_json::Value>>(
            reqwest::Method::GET,
            "/v1/admin/models",
            None::<&()>.as_ref(),
        )
        .await
    }

    /// Check health status of the local runtime.
    pub async fn health(&self) -> Result<serde_json::Value, IgrisError> {
        self.local_request::<serde_json::Value>(
            reqwest::Method::GET,
            "/v1/health",
            None::<&()>.as_ref(),
        )
        .await
    }

    // -- Containment observability ----------------------------------------

    async fn fetch_violations(&self) -> Vec<ViolationRecord> {
        self.local_request::<ViolationsResponse>(
            reqwest::Method::GET,
            "/v1/runtime/violations",
            None::<&()>.as_ref(),
        )
        .await
        .map(|r| r.violations)
        .unwrap_or_default()
    }

    /// Return the most recent violation record, or `None` if none exist or
    /// the endpoint is not reachable.
    pub async fn get_last_violation(&self) -> Option<ViolationRecord> {
        let mut records = self.fetch_violations().await;
        records.pop()
    }

    /// Poll for new violations every `interval`, calling `on_violation` for each
    /// new record.  Returns when `cancel` is dropped (or the future is dropped).
    ///
    /// # Example
    /// ```rust,ignore
    /// let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    /// tokio::spawn(runtime.stream_violations(std::time::Duration::from_secs(1), |v| {
    ///     println!("violation: {:?}", v);
    /// }, rx));
    /// ```
    pub async fn poll_violations<F>(
        &self,
        interval: std::time::Duration,
        mut on_violation: F,
        mut cancel: tokio::sync::oneshot::Receiver<()>,
    ) where
        F: FnMut(ViolationRecord),
    {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut ticker = tokio::time::interval(interval);
        loop {
            tokio::select! {
                _ = &mut cancel => break,
                _ = ticker.tick() => {
                    let records = self.fetch_violations().await;
                    for record in records {
                        if seen.insert(record.id.clone()) {
                            on_violation(record);
                        }
                    }
                }
            }
        }
    }
}
