use axum::{
    routing::get,
    Router,
    response::{IntoResponse, Response, Json},
    http::{header, StatusCode},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static ERROR_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize)]
struct MetricValue {
    value: u64,
    timestamp: u64,
}

#[derive(Debug, Serialize)]
struct LogEntry {
    timestamp: u64,
    level: String,
    message: String,
    metadata: HashMap<String, String>,
}

pub fn router() -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/traces", get(traces_handler))
        .route("/logs", get(logs_handler))
}

async fn metrics_handler() -> Response {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let metrics = format!(
        "# HELP request_count Total number of requests\n\
         # TYPE request_count counter\n\
         request_count {}\n\
         # HELP error_count Total number of errors\n\
         # TYPE error_count counter\n\
         error_count {}\n",
        REQUEST_COUNT.load(Ordering::Relaxed),
        ERROR_COUNT.load(Ordering::Relaxed)
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(metrics.into())
        .unwrap()
        .into_response()
}

async fn traces_handler() -> Response {
    // In a real implementation, this would return OpenTelemetry traces
    let traces = vec![
        ("trace_id_1", "GET /api/v1/models"),
        ("trace_id_2", "POST /api/v1/completions"),
    ];

    Json(traces).into_response()
}

async fn logs_handler() -> Response {
    let mut logs = Vec::new();
    logs.push(LogEntry {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        level: "INFO".to_string(),
        message: "Server started".to_string(),
        metadata: HashMap::new(),
    });

    Json(logs).into_response()
}

pub fn increment_request_count() {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn increment_error_count() {
    ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
}