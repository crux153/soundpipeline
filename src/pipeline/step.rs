use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait Step: Send + Sync {
    async fn execute(&self, working_dir: &Path) -> Result<()>;
    fn name(&self) -> &str;
}