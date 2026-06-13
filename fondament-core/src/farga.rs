use async_trait::async_trait;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct OrgContext { pub content: String }

#[derive(Debug, Clone)]
pub struct InitiativeContext { pub content: String }

#[derive(Debug, Clone)]
pub struct ProjectContext { pub content: String }

#[async_trait]
pub trait FargaReader: Send + Sync {
    async fn org_layer(&self, org: &str) -> Result<OrgContext>;
    async fn initiative_layer(&self, org: &str) -> Result<Vec<InitiativeContext>>;
    async fn project_layer(&self, project: &str) -> Result<ProjectContext>;
    async fn component_layer(&self, project: &str, path: &str) -> Result<ProjectContext>;
}
