//! Model management for the local Igris Runtime (BYOM - Bring Your Own Model).

use crate::errors::IgrisError;
use crate::runtime::Runtime;

/// Manages local runtime models.
pub struct ModelManager<'a> {
    runtime: &'a Runtime,
}

impl<'a> ModelManager<'a> {
    pub fn new(runtime: &'a Runtime) -> Self {
        Self { runtime }
    }

    /// Upload/load a GGUF model into the local runtime.
    pub async fn upload_model(
        &self,
        model_path: &str,
        model_id: Option<&str>,
    ) -> Result<serde_json::Value, IgrisError> {
        self.runtime.load_model(model_path, model_id).await
    }

    /// List models available on the local runtime.
    pub async fn list_local_models(&self) -> Result<Vec<serde_json::Value>, IgrisError> {
        self.runtime.list_models().await
    }

    /// Hot-swap to a different model on the runtime.
    pub async fn set_active_model(&self, model_id: &str) -> Result<serde_json::Value, IgrisError> {
        self.runtime.swap_model(model_id).await
    }
}
