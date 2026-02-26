//! Usage and audit management for Igris Inertial SDK.

use crate::client::IgrisClient;
use crate::errors::IgrisError;
use crate::types::*;

pub struct UsageManager<'a> {
    client: &'a IgrisClient,
}

impl<'a> UsageManager<'a> {
    pub fn new(client: &'a IgrisClient) -> Self {
        Self { client }
    }

    pub async fn current(&self) -> Result<Usage, IgrisError> {
        self.client.request::<Usage>(reqwest::Method::GET, "/v1/usage", None::<&()>.as_ref()).await
    }

    pub async fn history(&self) -> Result<UsageHistory, IgrisError> {
        self.client.request::<UsageHistory>(reqwest::Method::GET, "/v1/usage/history", None::<&()>.as_ref()).await
    }
}

pub struct AuditManager<'a> {
    client: &'a IgrisClient,
}

impl<'a> AuditManager<'a> {
    pub fn new(client: &'a IgrisClient) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<Vec<AuditEntry>, IgrisError> {
        #[derive(serde::Deserialize)]
        struct Resp { entries: Vec<AuditEntry> }
        let resp: Resp = self.client.request(reqwest::Method::GET, "/v1/audit", None::<&()>.as_ref()).await?;
        Ok(resp.entries)
    }
}
