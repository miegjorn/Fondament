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

// ── S3 registry client ────────────────────────────────────────────────────────

struct RegistryClient {
    client: aws_sdk_s3::Client,
    bucket: String,
    cache: tokio::sync::RwLock<std::collections::HashMap<String, (String, std::time::Instant)>>,
    ttl: std::time::Duration,
}

impl RegistryClient {
    fn new(endpoint: &str, bucket: &str) -> Self {
        use aws_sdk_s3::config::{Builder, Region};
        use aws_credential_types::Credentials;
        let creds = Credentials::new(
            std::env::var("AWS_ACCESS_KEY_ID").unwrap_or_else(|_| "occitan".into()),
            std::env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_else(|_| "occitan-dev".into()),
            None, None, "static",
        );
        let config = Builder::new()
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(creds)
            .force_path_style(true)
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .build();
        Self {
            client: aws_sdk_s3::Client::from_conf(config),
            bucket: bucket.to_string(),
            cache: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            ttl: std::time::Duration::from_secs(60),
        }
    }

    async fn get(&self, key: &str) -> anyhow::Result<String> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((content, ts)) = cache.get(key) {
                if ts.elapsed() < self.ttl {
                    return Ok(content.clone());
                }
            }
        }
        // Fetch from S3
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        let bytes = resp.body.collect().await?;
        let content = String::from_utf8(bytes.into_bytes().to_vec())?;
        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), (content.clone(), std::time::Instant::now()));
        }
        Ok(content)
    }

    /// Resolve id@version → YAML content. Version can be "latest", a semver, or absent (→ latest).
    async fn resolve_versioned(&self, id: &str, version: &str) -> anyhow::Result<String> {
        // "fondament/guilhem" stored as "fondament/guilhem/1.0.0.yaml";
        // "fondament/guilhem/latest" is a pointer file containing the canonical version string.
        let effective_version = if version == "latest" || version.is_empty() {
            let latest_key = format!("{}/latest", id);
            self.get(&latest_key).await?.trim().to_string()
        } else {
            version.to_string()
        };
        let key = format!("{}/{}.yaml", id, effective_version);
        self.get(&key).await
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    tree: Arc<DefinitionTree>,
    fondament: Arc<Fondament>,
    registry: Option<Arc<RegistryClient>>,
    definitions_path: PathBuf,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(definitions_path: PathBuf, farga_url: String) -> Router {
    let tree = DefinitionTree::load(&definitions_path)
        .expect("failed to load Fondament definitions");
    let farga = Arc::new(HttpFargaReader::new(farga_url));
    let fondament = Fondament::load(&definitions_path, farga, "occitan".into())
        .expect("failed to initialise Fondament");

    let registry_url = std::env::var("FONDAMENT_REGISTRY_URL").ok();
    let registry_bucket = std::env::var("FONDAMENT_REGISTRY_BUCKET")
        .unwrap_or_else(|_| "fondament-registry".into());
    let registry = registry_url.map(|url| Arc::new(RegistryClient::new(&url, &registry_bucket)));

    let state = AppState {
        tree: Arc::new(tree),
        fondament: Arc::new(fondament),
        registry,
        definitions_path,
    };

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/component-agents", get(list_component_agents))
        .route("/resolve/*id", get(resolve_definition))
        .route("/raw/*id", get(get_raw_definition))
        .route("/file/*path", get(get_raw_file))
        .with_state(state)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

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
    let id = id.trim_start_matches('/').to_string();

    // Check for @version suffix: "fondament/guilhem@1.0.0" or "fondament/guilhem@latest"
    if let Some((def_id, version)) = id.split_once('@') {
        if let Some(registry) = &s.registry {
            // @head = bypass registry, use local tree
            if version == "head" {
                return resolve_from_tree(&s, def_id).await;
            }
            return registry.resolve_versioned(def_id, version).await
                .map_err(|_| StatusCode::NOT_FOUND);
        }
    }

    // No version specifier: try registry @latest first, fall back to local tree
    if let Some(registry) = &s.registry {
        if let Ok(content) = registry.resolve_versioned(&id, "latest").await {
            return Ok(content);
        }
    }

    resolve_from_tree(&s, &id).await
}

async fn resolve_from_tree(s: &AppState, id: &str) -> Result<String, StatusCode> {
    let address: CompositionAddress = id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let resolved = s.fondament.resolve(&address).await.map_err(|_| StatusCode::NOT_FOUND)?;
    if resolved.system_prompt.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(resolved.system_prompt)
}

/// Return the raw YAML of a definition. Supports @version suffix (from registry)
/// or @head / no suffix (from local tree).
async fn get_raw_definition(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, StatusCode> {
    let id = id.trim_start_matches('/').to_string();

    if let Some((def_id, version)) = id.split_once('@') {
        if let Some(registry) = &s.registry {
            if version != "head" {
                return registry.resolve_versioned(def_id, version).await
                    .map_err(|_| StatusCode::NOT_FOUND);
            }
        }
    }

    let lookup_id = id.split('@').next().unwrap_or(&id);
    match s.tree.all().find(|d| d.id == lookup_id) {
        Some(d) => serde_yaml::to_string(d).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Serve a non-YAML file (e.g. system-defence.md) directly from the
/// definitions tree, live — no local vendoring on the caller's side.
/// DefinitionTree only indexes `.yaml` files, so files like
/// `definitions/fondament/system-defence.md` need this separate route.
/// Rejects any path that escapes `definitions_path` after normalization
/// (no `..` traversal), and any path containing a `..` component outright.
async fn get_raw_file(
    State(s): State<AppState>,
    Path(path): Path<String>,
) -> Result<String, StatusCode> {
    let requested = path.trim_start_matches('/');
    if requested.split('/').any(|part| part == "..") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let full_path = s.definitions_path.join(requested);
    let canonical_defs = s.definitions_path.canonicalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let canonical_target = full_path.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    if !canonical_target.starts_with(&canonical_defs) {
        return Err(StatusCode::BAD_REQUEST);
    }

    tokio::fs::read_to_string(&canonical_target).await.map_err(|_| StatusCode::NOT_FOUND)
}
