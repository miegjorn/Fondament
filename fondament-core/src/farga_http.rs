use async_trait::async_trait;
use crate::error::{FondamentError, Result};
use crate::farga::{FargaReader, InitiativeContext, OrgContext, ProjectContext};

pub struct HttpFargaReader {
    base_url: String,
    client: reqwest::Client,
}

impl HttpFargaReader {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn get_text(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.get(&url).send().await
            .map_err(|e| FondamentError::Farga(format!("HTTP GET {}: {}", url, e)))?;
        if resp.status().is_success() {
            resp.text().await
                .map_err(|e| FondamentError::Farga(format!("reading body from {}: {}", url, e)))
        } else if resp.status().as_u16() == 404 {
            Ok(String::new())
        } else {
            Err(FondamentError::Farga(format!("HTTP {} from {}", resp.status(), url)))
        }
    }
}

#[async_trait]
impl FargaReader for HttpFargaReader {
    async fn org_layer(&self, org: &str) -> Result<OrgContext> {
        let content = self.get_text(&format!("/context/org/{}", org)).await?;
        Ok(OrgContext { content })
    }

    async fn initiative_layer(&self, org: &str) -> Result<Vec<InitiativeContext>> {
        let url = format!("{}/context/initiatives/{}", self.base_url, org);
        let resp = self.client.get(&url).send().await
            .map_err(|e| FondamentError::Farga(format!("HTTP GET {}: {}", url, e)))?;
        if resp.status().as_u16() == 404 {
            return Ok(vec![]);
        }
        if !resp.status().is_success() {
            return Err(FondamentError::Farga(format!("HTTP {} from {}", resp.status(), url)));
        }
        let items: Vec<String> = resp.json().await
            .map_err(|e| FondamentError::Farga(format!("parsing initiatives from {}: {}", url, e)))?;
        Ok(items.into_iter().map(|content| InitiativeContext { content }).collect())
    }

    async fn project_layer(&self, project: &str) -> Result<ProjectContext> {
        let content = self.get_text(&format!("/context/project/{}", project)).await?;
        Ok(ProjectContext { content })
    }

    async fn component_layer(&self, project: &str, path: &str) -> Result<ProjectContext> {
        let content = self.get_text(&format!("/context/component/{}/{}", project, path)).await?;
        Ok(ProjectContext { content })
    }
}
