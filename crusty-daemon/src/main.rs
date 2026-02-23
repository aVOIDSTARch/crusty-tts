//! Crusty-TTS daemon: REST API, job management. Uses same execute_pipeline from crusty-core.

mod state;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use crusty_core::{execute_pipeline, validate_orchestration_types, Orchestration, PluginRegistry};
use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::var("CRUSTY_PORT").unwrap_or_else(|_| "7420".to_string());
    let plugins_dir = std::env::var("CRUSTY_PLUGINS").unwrap_or_else(|_| "plugins".to_string());
    let plugins_path = PathBuf::from(&plugins_dir);
    let registry = PluginRegistry::load_plugins(&plugins_path).unwrap_or_default();
    let app_state = AppState {
        registry: Arc::new(registry),
        plugins_base: plugins_path,
        jobs: Arc::new(state::JobState::default()),
    };

    let app = Router::new()
        .route("/plugins", get(list_plugins))
        .route("/plugins/:id", get(get_plugin))
        .route("/pipeline/validate", post(validate_pipeline))
        .route("/pipeline/run", post(run_pipeline))
        .route("/jobs/:id/status", get(job_status))
        .route("/jobs/:id/stream", get(job_stream))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("Crusty-TTS daemon listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(serde::Serialize)]
struct PluginInfo {
    name: String,
    r#type: String,
}

async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    let plugins: Vec<PluginInfo> = state
        .registry
        .as_ref()
        .all()
        .iter()
        .map(|p| PluginInfo {
            name: p.name.clone(),
            r#type: p.plugin_type.as_str().to_string(),
        })
        .collect();
    Json(plugins)
}

async fn get_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.registry.as_ref().get(&id) {
        Some(p) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "name": p.name,
                "type": p.plugin_type.as_str(),
                "path": p.path,
                "options": p.options,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "plugin not found"})),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ValidateRequest {
    orchestration: String,
}

async fn validate_pipeline(
    State(state): State<AppState>,
    Json(body): Json<ValidateRequest>,
) -> impl IntoResponse {
    match Orchestration::from_toml(&body.orchestration) {
        Ok(orch) => {
            let plugin_base = &state.plugins_base;
            let mut errors = Vec::new();
            if let Some(ref pre) = orch.pre_processors {
                for p in pre {
                    let path = plugin_base.join(&p.module);
                    if !path.join("run.sh").exists() && !path.join("run.py").exists() {
                        errors.push(format!("pre-processor {} missing run.sh/run.py", p.name));
                    }
                }
            }
            let tts_path = plugin_base.join(&orch.tts.module);
            if !tts_path.join("run.sh").exists() && !tts_path.join("run.py").exists() {
                errors.push(format!("TTS {} missing run.sh/run.py", orch.tts.name));
            }
            if let Err(e) = validate_orchestration_types(&orch, plugin_base) {
                errors.push(format!("type validation: {}", e));
            }
            if errors.is_empty() {
                (StatusCode::OK, Json(serde_json::json!({"valid": true})))
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"valid": false, "errors": errors})),
                )
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"valid": false, "error": e.to_string()})),
        ),
    }
}

#[derive(serde::Deserialize)]
struct RunRequest {
    orchestration: String,
    input_path: Option<String>,
}

async fn run_pipeline(
    State(state): State<AppState>,
    Json(body): Json<RunRequest>,
) -> impl IntoResponse {
    let orch = match Orchestration::from_toml(&body.orchestration) {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            );
        }
    };
    let mut orch = orch;
    if let Some(ref p) = body.input_path {
        orch.input.source = p.clone();
    }
    let job_id = uuid::Uuid::new_v4().to_string();
    let response_job_id = job_id.clone();
    state.jobs.set_status(&job_id, state::JobStatus::Running);
    let plugin_base = state.plugins_base.clone();
    let jobs = Arc::clone(&state.jobs);
    tokio::task::spawn_blocking(move || {
        match execute_pipeline(&orch, &plugin_base) {
            Ok(audio) => {
                jobs.set_completed(&job_id, audio);
            }
            Err(e) => {
                jobs.set_failed(&job_id, e.to_string());
            }
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id": response_job_id})),
    )
}

async fn job_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.jobs.get_status(&id) {
        Some(s) => (
            StatusCode::OK,
            Json(serde_json::json!({"job_id": id, "status": s})),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "job not found"})),
        ),
    }
}

async fn job_stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.jobs.get_output(&id) {
        Some(audio) => {
            let mut res = Response::new(Body::from(audio));
            res.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/octet-stream"),
            );
            res
        }
        None => (StatusCode::NOT_FOUND, "job not found").into_response(),
    }
}
