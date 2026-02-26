//! Fleet management for Igris Inertial SDK.

use crate::client::IgrisClient;
use crate::errors::IgrisError;
use crate::types::*;

pub struct FleetManager<'a> {
    client: &'a IgrisClient,
}

impl<'a> FleetManager<'a> {
    pub fn new(client: &'a IgrisClient) -> Self {
        Self { client }
    }

    pub async fn register(&self, config: &serde_json::Value) -> Result<FleetAgent, IgrisError> {
        self.client.request(reqwest::Method::POST, "/api/fleet/register", Some(config)).await
    }

    pub async fn telemetry(&self, fleet_id: &str, data: &serde_json::Value) -> Result<(), IgrisError> {
        self.client.send_json_no_response(
            reqwest::Method::POST,
            &format!("/api/fleet/{}/telemetry", fleet_id),
            data,
        ).await
    }

    pub async fn agents(&self) -> Result<Vec<FleetAgent>, IgrisError> {
        #[derive(serde::Deserialize)]
        struct Resp { agents: Vec<FleetAgent> }
        let resp: Resp = self.client.request(reqwest::Method::GET, "/api/fleet/agents", None::<&()>.as_ref()).await?;
        Ok(resp.agents)
    }

    pub async fn health(&self) -> Result<FleetHealth, IgrisError> {
        self.client.request::<FleetHealth>(reqwest::Method::GET, "/api/fleet/health", None::<&()>.as_ref()).await
    }
}
