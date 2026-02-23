//! Crusty-TTS daemon library: app builder for testing and serving.

mod state;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use crusty_core::{execute_pipeline, validate_orchestration_types, Orchestration};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// Build the axum Router with the given state (used by main and tests).
pub fn build_app(state: state::AppState) -> Router {
    Router::new()
        .route("/plugins", get(list_plugins))
        .route("/plugins/:id", get(get_plugin))
        .route("/pipeline/validate", post(validate_pipeline))
        .route("/pipeline/run", post(run_pipeline))
        .route("/jobs/:id/status", get(job_status))
        .route("/jobs/:id/stream", get(job_stream))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn list_plugins(State(state): State<state::AppState>) -> impl IntoResponse {
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

#[derive(serde::Serialize)]
struct PluginInfo {
    name: String,
    r#type: String,
}

async fn get_plugin(
    State(state): State<state::AppState>,
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
    State(state): State<state::AppState>,
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
    State(state): State<state::AppState>,
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
    State(state): State<state::AppState>,
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
    State(state): State<state::AppState>,
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

pub use state::{AppState, JobState, JobStatus};

#[cfg(test)]
mod tests {
    use super::*;
    use crusty_core::PluginRegistry;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_app_state() -> state::AppState {
        state::AppState {
            registry: Arc::new(PluginRegistry::load_plugins(PathBuf::from("plugins").as_path()).unwrap_or_default()),
            plugins_base: PathBuf::from("."),
            jobs: Arc::new(JobState::default()),
        }
    }

    #[tokio::test]
    async fn get_plugins_returns_200() {
        let app = build_app(test_app_state());
        let req = Request::builder().uri("/plugins").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_plugin_404_for_unknown() {
        let app = build_app(test_app_state());
        let req = Request::builder().uri("/plugins/nonexistent").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_pipeline_invalid_toml_returns_400() {
        let app = build_app(test_app_state());
        let req = Request::builder()
            .method("POST")
            .uri("/pipeline/validate")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"orchestration": "invalid ["}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn job_status_404_for_unknown() {
        let app = build_app(test_app_state());
        let req = Request::builder().uri("/jobs/nonexistent-id/status").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn job_stream_404_for_unknown() {
        let app = build_app(test_app_state());
        let req = Request::builder().uri("/jobs/nonexistent-id/stream").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_pipeline_valid_minimal_returns_200_or_errors() {
        let app = build_app(test_app_state());
        let orch = r#"
[meta]
name = "t"
version = "0.1"
author = "a"
[input]
type = "text"
source = "in.txt"
[tts]
name = "tts"
module = "plugins/tts"
[output]
type = "file"
path = "out.bin"
"#;
        let req = Request::builder()
            .method("POST")
            .uri("/pipeline/validate")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({ "orchestration": orch }).to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        // May be 200 (valid) if plugins/tts exists with run.sh, or 400 (errors)
        assert!(res.status() == StatusCode::OK || res.status() == StatusCode::BAD_REQUEST);
    }
}