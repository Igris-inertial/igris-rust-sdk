//! Main Igris Inertial client.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::errors::IgrisError;
use crate::fleet::FleetManager;
use crate::providers::ProviderManager;
use crate::types::*;
use crate::usage::{AuditManager, UsageManager};
use crate::vault::VaultManager;

/// Client for the Igris Inertial AI inference gateway.
pub struct IgrisClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    #[allow(dead_code)]
    tenant_id: Option<String>,
}

/// Builder for configuring an IgrisClient.
pub struct IgrisClientBuilder {
    base_url: String,
    api_key: Option<String>,
    timeout: std::time::Duration,
    tenant_id: Option<String>,
}

impl IgrisClientBuilder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            timeout: std::time::Duration::from_secs(30),
            tenant_id: None,
        }
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    pub fn build(self) -> Result<IgrisClient, IgrisError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(ref key) = self.api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", key))
                    .map_err(|e| IgrisError::Api { message: e.to_string(), status_code: 0 })?,
            );
        }
        if let Some(ref tid) = self.tenant_id {
            headers.insert(
                "X-Tenant-ID",
                HeaderValue::from_str(tid)
                    .map_err(|e| IgrisError::Api { message: e.to_string(), status_code: 0 })?,
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(self.timeout)
            .build()?;

        Ok(IgrisClient {
            http,
            base_url: self.base_url.trim_end_matches('/').to_string(),
            api_key: self.api_key,
            tenant_id: self.tenant_id,
        })
    }
}

impl IgrisClient {
    /// Create a new client with builder pattern.
    pub fn builder(base_url: impl Into<String>) -> IgrisClientBuilder {
        IgrisClientBuilder::new(base_url)
    }

    /// Simple constructor.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self, IgrisError> {
        Self::builder(base_url).api_key(api_key).build()
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub(crate) async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<T, IgrisError> {
        let mut req = self.http.request(method, self.url(path));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status().as_u16();

        if status == 401 || status == 403 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Authentication { message: text, status_code: status });
        }
        if status == 429 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::RateLimit { message: text });
        }
        if status == 400 || status == 422 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Validation { message: text, status_code: status });
        }
        if status >= 400 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Api { message: text, status_code: status });
        }

        let data = resp.json().await?;
        Ok(data)
    }

    pub(crate) async fn request_no_body(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<(), IgrisError> {
        let resp = self.http.request(method, self.url(path)).send().await?;
        let status = resp.status().as_u16();

        if status == 401 || status == 403 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Authentication { message: text, status_code: status });
        }
        if status >= 400 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Api { message: text, status_code: status });
        }
        Ok(())
    }

    pub(crate) async fn send_json_no_response(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<(), IgrisError> {
        let resp = self.http.request(method, self.url(path)).json(body).send().await?;
        let status = resp.status().as_u16();

        if status == 401 || status == 403 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Authentication { message: text, status_code: status });
        }
        if status >= 400 {
            let text = resp.text().await.unwrap_or_default();
            return Err(IgrisError::Api { message: text, status_code: status });
        }
        Ok(())
    }

    // ── Auth ──

    pub async fn login(&self, api_key: Option<&str>) -> Result<serde_json::Value, IgrisError> {
        let key = api_key.or(self.api_key.as_deref()).unwrap_or_default();
        let body = serde_json::json!({"api_key": key});
        self.request(reqwest::Method::POST, "/v1/auth/login", Some(&body)).await
    }

    pub async fn refresh_token(&self) -> Result<serde_json::Value, IgrisError> {
        self.request::<serde_json::Value>(reqwest::Method::POST, "/v1/auth/refresh", None::<&()>.as_ref()).await
    }

    pub async fn logout(&self) -> Result<(), IgrisError> {
        self.request_no_body(reqwest::Method::POST, "/v1/auth/logout").await
    }

    // ── Inference ──

    pub async fn infer(&self, request: &InferRequest) -> Result<InferResponse, IgrisError> {
        self.request(reqwest::Method::POST, "/v1/infer", Some(request)).await
    }

    pub async fn chat_completion(&self, request: &InferRequest) -> Result<InferResponse, IgrisError> {
        self.request(reqwest::Method::POST, "/v1/chat/completions", Some(request)).await
    }

    pub async fn list_models(&self) -> Result<ModelsResponse, IgrisError> {
        self.request::<ModelsResponse>(reqwest::Method::GET, "/v1/models", None::<&()>.as_ref()).await
    }

    pub async fn health(&self) -> Result<HealthResponse, IgrisError> {
        self.request::<HealthResponse>(reqwest::Method::GET, "/v1/health", None::<&()>.as_ref()).await
    }

    pub async fn provider_stats(&self) -> Result<serde_json::Value, IgrisError> {
        self.request::<serde_json::Value>(reqwest::Method::GET, "/v1/providers/stats", None::<&()>.as_ref()).await
    }

    // ── Sub-managers ──

    pub fn providers(&self) -> ProviderManager<'_> {
        ProviderManager::new(self)
    }

    pub fn vault(&self) -> VaultManager<'_> {
        VaultManager::new(self)
    }

    pub fn fleet(&self) -> FleetManager<'_> {
        FleetManager::new(self)
    }

    pub fn usage(&self) -> UsageManager<'_> {
        UsageManager::new(self)
    }

    pub fn audit(&self) -> AuditManager<'_> {
        AuditManager::new(self)
    }

    // ── BYOK Convenience Aliases ──

    /// Store a provider API key in the vault.
    pub async fn upload_key(&self, provider: &str, api_key: &str) -> Result<VaultKey, IgrisError> {
        self.vault().store(&VaultStoreRequest {
            provider: provider.to_string(),
            api_key: api_key.to_string(),
            config: None,
        }).await
    }

    /// Rotate a provider API key.
    pub async fn rotate_key(&self, provider: &str) -> Result<VaultKey, IgrisError> {
        self.vault().rotate(provider).await
    }

    /// List all stored vault keys.
    pub async fn list_keys(&self) -> Result<Vec<VaultKey>, IgrisError> {
        self.vault().list().await
    }
}
