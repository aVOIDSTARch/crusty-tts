//! Crusty-TTS daemon: REST API, job management. Uses same execute_pipeline from crusty-core.

use crusty_daemon::{build_app, AppState};
use crusty_core::PluginRegistry;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::var("CRUSTY_PORT").unwrap_or_else(|_| "7420".to_string());
    let plugins_dir = std::env::var("CRUSTY_PLUGINS").unwrap_or_else(|_| "plugins".to_string());
    let plugins_path = PathBuf::from(&plugins_dir);
    let registry = PluginRegistry::load_plugins(&plugins_path).unwrap_or_default();
    let app_state = AppState {
        registry: Arc::new(registry),
        plugins_base: plugins_path,
        jobs: Arc::new(crusty_daemon::JobState::default()),
    };

    let app = build_app(app_state);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("Crusty-TTS daemon listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
