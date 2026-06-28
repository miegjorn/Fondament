use std::path::PathBuf;
use fondament_server::router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let definitions_path = PathBuf::from(
        std::env::var("FONDAMENT_DEFINITIONS_PATH").unwrap_or_else(|_| "definitions".into())
    );
    let farga_url = std::env::var("FARGA_URL")
        .unwrap_or_else(|_| "http://farga:7500".into());
    let port = std::env::var("FONDAMENT_PORT").unwrap_or_else(|_| "7800".into());

    let app = router(definitions_path, farga_url);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("fondament-server listening on :{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
