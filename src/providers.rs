//! Provider management for Igris Inertial SDK.

use crate::client::IgrisClient;
use crate::errors::IgrisError;
use crate::types::*;

pub struct ProviderManager<'a> {
    client: &'a IgrisClient,
}

impl<'a> ProviderManager<'a> {
    pub fn new(client: &'a IgrisClient) -> Self {
        Self { client }
    }

    pub async fn register(&self, config: &ProviderConfig) -> Result<Provider, IgrisError> {
        self.client.request(reqwest::Method::POST, "/v1/providers/register", Some(config)).await
    }

    pub async fn list(&self) -> Result<Vec<Provider>, IgrisError> {
        #[derive(serde::Deserialize)]
        struct Resp { providers: Vec<Provider> }
        let resp: Resp = self.client.request(reqwest::Method::GET, "/v1/providers", None::<&()>.as_ref()).await?;
        Ok(resp.providers)
    }

    pub async fn test(&self, config: &ProviderConfig) -> Result<TestResult, IgrisError> {
        self.client.request(reqwest::Method::POST, "/v1/providers/test", Some(config)).await
    }

    pub async fn update(&self, id: &str, config: &serde_json::Value) -> Result<Provider, IgrisError> {
        self.client.request(reqwest::Method::PUT, &format!("/v1/providers/{}", id), Some(config)).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), IgrisError> {
        self.client.request_no_body(reqwest::Method::DELETE, &format!("/v1/providers/{}", id)).await
    }

    pub async fn health(&self, id: &str) -> Result<HealthStatus, IgrisError> {
        self.client.request::<HealthStatus>(reqwest::Method::GET, &format!("/v1/providers/{}/health", id), None::<&()>.as_ref()).await
    }
}
