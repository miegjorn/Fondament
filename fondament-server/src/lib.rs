use std::{path::PathBuf, sync::Arc};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use fondament_core::{
    address::CompositionAddress,
    farga_http::HttpFargaReader,
    fondament::Fondament,
    tree::DefinitionTree,
};
use serde_json::Value;

#[derive(Clone)]
pub struct AppState {
    tree: Arc<DefinitionTree>,
    fondament: Arc<Fondament>,
}

pub fn router(definitions_path: PathBuf, farga_url: String) -> Router {
    let tree = DefinitionTree::load(&definitions_path)
        .expect("failed to load Fondament definitions");
    let farga = Arc::new(HttpFargaReader::new(farga_url));
    let fondament = Fondament::load(&definitions_path, farga, "occitan".into())
        .expect("failed to initialise Fondament");

    let state = AppState {
        tree: Arc::new(tree),
        fondament: Arc::new(fondament),
    };

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/component-agents", get(list_component_agents))
        .route("/resolve/*id", get(resolve_definition))
        .with_state(state)
}

async fn list_component_agents(State(s): State<AppState>) -> Json<Vec<Value>> {
    let agents = s.tree
        .all()
        .filter(|d| d.kind == "component-agent")
        .map(|d| serde_json::json!({
            "id": d.id,
            "component": d.component.as_deref().unwrap_or("")
        }))
        .collect();
    Json(agents)
}

async fn resolve_definition(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, StatusCode> {
    // axum may include a leading slash in the wildcard capture; strip it
    let id = id.trim_start_matches('/').to_string();
    let address: CompositionAddress = id.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let resolved = s.fondament.resolve(&address).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if resolved.system_prompt.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(resolved.system_prompt)
}
