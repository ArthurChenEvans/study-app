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
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use chrono::{Local, NaiveDate};
use serde_json::{Value, json};
use study_api::App;
use tower_http::services::{ServeDir, ServeFile};

pub use dispatch::{Reply, dispatch};

#[derive(Clone)]
struct Shared {
    app: Arc<Mutex<App>>,
    today: fn() -> i64,
    /// Host names the server answers to, besides this computer's own.
    hosts: Arc<Vec<String>>,
}

/// The same policy as the desktop app (src-tauri/tauri.conf.json), plus what only matters
/// in a browser: nothing may frame the page, and forms can't post anywhere.
pub const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; script-src 'self'; worker-src 'self' blob:; connect-src 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'none'; base-uri 'none'";

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
    router_for(app, dist, today, &[])
}

/// The same, also answering to `hosts`: names other than localhost that you reach the
/// server by (`STUDY_HOSTS`).
pub fn router_for(app: App, dist: &Path, today: fn() -> i64, hosts: &[String]) -> Router {
    let shared = Shared { app: Arc::new(Mutex::new(app)), today, hosts: Arc::new(hosts.iter().map(|host| host.trim().to_lowercase()).filter(|host| !host.is_empty()).collect()) };
    let frontend = ServeDir::new(dist).fallback(ServeFile::new(dist.join("index.html")));
    let api = Router::new().route("/api/{command}", post(command)).layer(middleware::from_fn_with_state(shared.clone(), only_our_own_page));
    Router::new()
        .merge(api)
        .fallback_service(frontend)
        .with_state(shared)
        .layer(axum::extract::DefaultBodyLimit::max(200 * 1024 * 1024))
        .layer(middleware::from_fn(security_headers))
}

/// `STUDY_HOSTS` as a list: names separated by commas, blanks ignored.
pub fn host_list(setting: &str) -> Vec<String> {
    setting.split(',').map(|host| host.trim().to_lowercase()).filter(|host| !host.is_empty()).collect()
}

/// The host part of `localhost:7392`, `[::1]:7392` or `study.lan`, in lower case.
fn host_name(host: &str) -> String {
    let host = host.trim().to_lowercase();
    match host.strip_prefix('[') {
        Some(rest) => format!("[{}", rest.split_inclusive(']').next().unwrap_or(rest)),
        None => host.split(':').next().unwrap_or_default().to_string(),
    }
}

/// Why a request to the API is refused, if it is. There are no logins, so the server
/// trusts whoever reaches it, and any web page open in the same browser can reach
/// 127.0.0.1. Three checks keep those out:
/// - the Host must be this computer (or a name in `STUDY_HOSTS`), which stops a site that
///   points its own name at 127.0.0.1 (DNS rebinding)
/// - an Origin, when the browser sends one, must be this same host and port
/// - the body must be sent as JSON, which a page on another site can't do without
///   asking first (a CORS preflight, which this server never grants)
fn refusal(headers: &HeaderMap, hosts: &[String]) -> Option<&'static str> {
    let text = |name: header::HeaderName| headers.get(name).and_then(|value| value.to_str().ok()).map(|value| value.trim().to_lowercase());
    let host = text(header::HOST);
    if let Some(host) = &host {
        let name = host_name(host);
        if !["localhost", "127.0.0.1", "[::1]"].contains(&name.as_str()) && !hosts.contains(&name) {
            return Some("this server only answers on localhost. To reach it by another name, list that name in STUDY_HOSTS");
        }
    }
    if let Some(origin) = text(header::ORIGIN) {
        let same = host.as_ref().is_some_and(|host| origin.strip_prefix("http://").or_else(|| origin.strip_prefix("https://")) == Some(host.as_str()));
        if !same {
            return Some("requests from other websites are refused");
        }
    }
    if headers.get("sec-fetch-site").and_then(|value| value.to_str().ok()).is_some_and(|site| !["same-origin", "none"].contains(&site.trim())) {
        return Some("requests from other websites are refused");
    }
    let json = text(header::CONTENT_TYPE).is_some_and(|kind| kind.split(';').next().unwrap_or_default().trim() == "application/json");
    if !json {
        return Some("send the arguments as JSON (content-type: application/json)");
    }
    None
}

async fn only_our_own_page(State(shared): State<Shared>, request: Request<axum::body::Body>, next: Next) -> Response {
    match refusal(request.headers(), &shared.hosts) {
        Some(why) => error(StatusCode::FORBIDDEN, why),
        None => next.run(request).await,
    }
}

async fn security_headers(request: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(CONTENT_SECURITY_POLICY));
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert("cross-origin-resource-policy", HeaderValue::from_static("same-origin"));
    response
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
        // A panic in an earlier command doesn't lock the app for good (see study_api::locked).
        let mut app = study_api::locked(&shared.app);
        dispatch(&mut app, &name, &args, today)
    })
    .await;
    match result {
        Ok(Ok(Reply::Json(value))) => axum::Json(value).into_response(),
        Ok(Ok(Reply::Bytes(bytes))) => ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response(),
        Ok(Err(failure)) => error(StatusCode::BAD_REQUEST, failure.message),
        Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "the app hit an internal error. Nothing was lost, try that again"),
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
    /// Where backups go, `STUDY_BACKUPS`. Defaults to a `backups` folder beside the database.
    pub backups: PathBuf,
    pub seed: bool,
    /// `STUDY_HOSTS`: names besides localhost the server answers to, separated by commas.
    pub hosts: Vec<String>,
    /// `STUDY_SEED=large` seeds the heavy profile for testing how screens cope with a lot of data.
    pub large: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_string());
        Self {
            backups: std::env::var("STUDY_BACKUPS").map(PathBuf::from).unwrap_or_else(|_| {
                let db = PathBuf::from(var("STUDY_DB", "study-web.sqlite"));
                db.parent().filter(|parent| !parent.as_os_str().is_empty()).map_or_else(|| PathBuf::from("backups"), |parent| parent.join("backups"))
            }),
            db: var("STUDY_DB", "study-web.sqlite").into(),
            dist: var("STUDY_DIST", "dist").into(),
            address: var("STUDY_ADDRESS", "127.0.0.1:7392"),
            seed: matches!(var("STUDY_SEED", "").as_str(), "1" | "true" | "yes" | "large"),
            large: var("STUDY_SEED", "") == "large",
            hosts: host_list(&var("STUDY_HOSTS", "")),
        }
    }
}
