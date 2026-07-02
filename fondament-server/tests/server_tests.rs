use axum::{body::Body, http::{Request, StatusCode}};
use std::path::PathBuf;
use tower::ServiceExt;

fn make_app(definitions_path: PathBuf) -> axum::Router {
    fondament_server::router(definitions_path, "http://farga-does-not-exist:7500".into())
}

#[tokio::test]
async fn health_returns_ok() {
    let dir = tempfile::tempdir().unwrap();
    let app = make_app(dir.path().to_path_buf());
    let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_component_agents_returns_only_component_agent_kind() {
    let dir = tempfile::tempdir().unwrap();
    let fondament_dir = dir.path().join("fondament");
    std::fs::create_dir_all(&fondament_dir).unwrap();

    std::fs::write(fondament_dir.join("amassada-agent.yaml"), r#"
id: fondament/amassada-agent
kind: component-agent
component: amassada
context: "You are Amassada."
"#).unwrap();

    std::fs::write(fondament_dir.join("guilhem.yaml"), r#"
id: fondament/guilhem
kind: role
context: "You are Guilhem."
"#).unwrap();

    let app = make_app(dir.path().to_path_buf());
    let req = Request::builder().uri("/component-agents").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let agents: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["id"], "fondament/amassada-agent");
    assert_eq!(agents[0]["component"], "amassada");
}

#[tokio::test]
async fn resolve_returns_system_prompt_for_known_id() {
    let dir = tempfile::tempdir().unwrap();
    let fondament_dir = dir.path().join("fondament");
    std::fs::create_dir_all(&fondament_dir).unwrap();
    std::fs::write(fondament_dir.join("amassada-agent.yaml"), r#"
id: fondament/amassada-agent
kind: component-agent
component: amassada
context: "You are the Amassada session engine agent."
"#).unwrap();

    let app = make_app(dir.path().to_path_buf());
    let req = Request::builder()
        .uri("/resolve/fondament/amassada-agent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("Amassada session engine agent"));
}

#[tokio::test]
async fn resolve_returns_404_for_unknown_id() {
    let dir = tempfile::tempdir().unwrap();
    let app = make_app(dir.path().to_path_buf());
    let req = Request::builder()
        .uri("/resolve/fondament/does-not-exist")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn file_returns_raw_content_for_non_yaml_file() {
    let dir = tempfile::tempdir().unwrap();
    let fondament_dir = dir.path().join("fondament");
    std::fs::create_dir_all(&fondament_dir).unwrap();
    std::fs::write(fondament_dir.join("system-defence.md"), "# System Defence\n\nA-1: ...\n").unwrap();

    let app = make_app(dir.path().to_path_buf());
    let req = Request::builder()
        .uri("/file/fondament/system-defence.md")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("A-1"));
}

#[tokio::test]
async fn file_returns_404_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("fondament")).unwrap();
    let app = make_app(dir.path().to_path_buf());
    let req = Request::builder()
        .uri("/file/fondament/does-not-exist.md")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn file_rejects_path_traversal_outside_definitions_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("fondament")).unwrap();
    // A real secret file living outside the definitions root.
    std::fs::write(dir.path().join("secret.txt"), "should never be served").unwrap();

    let app = make_app(dir.path().join("fondament"));
    let req = Request::builder()
        .uri("/file/..%2Fsecret.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_ne!(resp.status(), StatusCode::OK);
}
