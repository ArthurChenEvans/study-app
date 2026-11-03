//! The Study App over HTTP. `POST /api/<command>` takes the same JSON arguments as the
//! desktop app's Tauri commands and returns JSON (or bytes for images and PDFs). Errors
//! come back as `{ message }` with status 400, like the desktop app's rejected promises.
//! Everything else is the built frontend from `dist/`.

pub mod dispatch;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use chrono::{Local, NaiveDate};
use serde_json::{Value, json};
use study_api::{ApiError, App};
use tower_http::services::{ServeDir, ServeFile};

pub use dispatch::{Reply, dispatch};

#[derive(Clone)]
struct Shared {
    app: Arc<Mutex<App>>,
    today: fn() -> i64,
}

/// Today as days since 1970-01-01 in local time, the same as the desktop app.
pub fn today() -> i64 {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date");
    (Local::now().date_naive() - epoch).num_days()
}

/// Now, for sessions and struggles: the day, minutes after midnight and unix seconds.
pub fn clock() -> study_api::practice::Clock {
    use chrono::Timelike;
    let now = Local::now();
    study_api::practice::Clock { day: today(), minute: now.hour() * 60 + now.minute(), unix: now.timestamp() }
}

/// The routes, with the frontend served from `dist` (index.html for anything unknown).
pub fn router(app: App, dist: &Path, today: fn() -> i64) -> Router {
    let shared = Shared { app: Arc::new(Mutex::new(app)), today };
    let frontend = ServeDir::new(dist).fallback(ServeFile::new(dist.join("index.html")));
    Router::new()
        .route("/api/{command}", post(command))
        .fallback_service(frontend)
        .with_state(shared)
        .layer(axum::extract::DefaultBodyLimit::max(200 * 1024 * 1024))
}

fn error(status: StatusCode, message: impl ToString) -> Response {
    (status, axum::Json(json!({ "message": message.to_string() }))).into_response()
}

async fn command(State(shared): State<Shared>, UrlPath(name): UrlPath<String>, body: Bytes) -> Response {
    let args: Value = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body) {
            Ok(args) => args,
            Err(_) => return error(StatusCode::BAD_REQUEST, "the request wasn't valid JSON"),
        }
    };
    if name == "slides_to_pdf" {
        return slides(args).await;
    }
    let today = (shared.today)();
    let result = tokio::task::spawn_blocking(move || {
        let mut app = shared.app.lock().map_err(|_| ApiError::new("the app hit an internal error, please restart it"))?;
        dispatch(&mut app, &name, &args, today)
    })
    .await;
    match result {
        Ok(Ok(Reply::Json(value))) => axum::Json(value).into_response(),
        Ok(Ok(Reply::Bytes(bytes))) => ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response(),
        Ok(Err(failure)) => error(StatusCode::BAD_REQUEST, failure.message),
        Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "the app hit an internal error, please restart it"),
    }
}

async fn slides(args: Value) -> Response {
    use base64::Engine;
    let name = args.get("name").and_then(Value::as_str).unwrap_or("slides.pptx").to_string();
    let Some(bytes) = args.get("data").and_then(Value::as_str).and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok()) else {
        return error(StatusCode::BAD_REQUEST, "the slides couldn't be read, try opening the file again");
    };
    let converted = tokio::task::spawn_blocking(move || study_api::convert::slides_to_pdf(&bytes, &name, None, std::time::Duration::from_secs(90))).await;
    match converted {
        Ok(Ok(pdf)) => ([(header::CONTENT_TYPE, "application/pdf")], pdf).into_response(),
        Ok(Err(failure)) => error(StatusCode::BAD_REQUEST, failure),
        Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "the conversion stopped unexpectedly"),
    }
}

/// Where things live, from the environment, with defaults for running from the repo.
pub struct Config {
    pub db: PathBuf,
    pub dist: PathBuf,
    pub address: String,
    pub seed: bool,
    /// `STUDY_SEED=large` seeds the heavy profile for testing how screens cope with a lot of data.
    pub large: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_string());
        Self {
            db: var("STUDY_DB", "study-web.sqlite").into(),
            dist: var("STUDY_DIST", "dist").into(),
            address: var("STUDY_ADDRESS", "127.0.0.1:7392"),
            seed: matches!(var("STUDY_SEED", "").as_str(), "1" | "true" | "yes" | "large"),
            large: var("STUDY_SEED", "") == "large",
        }
    }
}
