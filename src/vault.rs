//! BYOK vault management for Igris Inertial SDK.

use crate::client::IgrisClient;
use crate::errors::IgrisError;
use crate::types::*;

pub struct VaultManager<'a> {
    client: &'a IgrisClient,
}

impl<'a> VaultManager<'a> {
    pub fn new(client: &'a IgrisClient) -> Self {
        Self { client }
    }

    pub async fn store(&self, request: &VaultStoreRequest) -> Result<VaultKey, IgrisError> {
        self.client.request(reqwest::Method::POST, "/v1/vault/keys", Some(request)).await
    }

    pub async fn list(&self) -> Result<Vec<VaultKey>, IgrisError> {
        #[derive(serde::Deserialize)]
        struct Resp { keys: Vec<VaultKey> }
        let resp: Resp = self.client.request(reqwest::Method::GET, "/v1/vault/keys", None::<&()>.as_ref()).await?;
        Ok(resp.keys)
    }

    pub async fn rotate(&self, provider: &str) -> Result<VaultKey, IgrisError> {
        self.client.request::<VaultKey>(reqwest::Method::POST, &format!("/v1/vault/keys/{}/rotate", provider), None::<&()>.as_ref()).await
    }

    pub async fn delete(&self, provider: &str) -> Result<(), IgrisError> {
        self.client.request_no_body(reqwest::Method::DELETE, &format!("/v1/vault/keys/{}", provider)).await
    }
}
