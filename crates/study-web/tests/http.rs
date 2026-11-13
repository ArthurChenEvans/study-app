//! The web server answers the same commands as the desktop app, over HTTP.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use study_api::App;
use study_store::Store;
use study_web::{Reply, dispatch, router};
use tower::ServiceExt;

const TODAY: i64 = 20_760;

fn dist() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("study-web-dist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<title>Study App</title>").unwrap();
    dir
}

fn server() -> axum::Router {
    router(App::with_seed(Store::in_memory().unwrap(), 1), &dist(), || TODAY)
}

async fn call(server: &axum::Router, command: &str, args: Value) -> (StatusCode, String, Vec<u8>) {
    let request = Request::post(format!("/api/{command}")).header("content-type", "application/json").body(Body::from(args.to_string())).unwrap();
    let response = server.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let kind = response.headers().get("content-type").map(|value| value.to_str().unwrap().to_string()).unwrap_or_default();
    (status, kind, response.into_body().collect().await.unwrap().to_bytes().to_vec())
}

fn json_of(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}

#[test]
fn every_command_the_frontend_calls_is_handled() {
    let api = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/lib/api.ts")).unwrap();
    let names: Vec<&str> = api.split("invoke<").skip(1).filter_map(|rest| rest.split_once("(\"").and_then(|(_, rest)| rest.split_once('"')).map(|(name, _)| name)).collect();
    assert!(names.len() > 60, "found {} commands", names.len());
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    for name in names.into_iter().filter(|name| *name != "slides_to_pdf") {
        if let Err(error) = dispatch(&mut app, name, &json!({}), TODAY) {
            assert!(!error.message.contains("not found"), "{name}: {}", error.message);
        }
    }
}

#[test]
fn unknown_commands_say_so() {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    let Err(error) = dispatch(&mut app, "make_coffee", &json!({}), TODAY) else { panic!("expected an error") };
    assert_eq!(error.message, "Command make_coffee not found");
}

#[tokio::test]
async fn commands_take_json_and_give_json() {
    let server = server();
    let (status, _, body) = call(&server, "create_module", json!({ "name": "Networks" })).await;
    assert_eq!(status, StatusCode::OK);
    let module = json_of(&body);
    let (_, _, body) = call(&server, "create_topic", json!({ "name": "OSI model", "module": module })).await;
    let topic = json_of(&body);
    let (_, _, body) = call(&server, "topics", json!({})).await;
    assert_eq!(json_of(&body), json!([{ "id": topic, "name": "OSI model", "module": module, "tags": [], "question_count": 0, "short_count": 0 }]));
    let (_, _, body) = call(&server, "today_day", json!({})).await;
    assert_eq!(json_of(&body), json!(TODAY));
    let (_, _, body) = call(&server, "leave_review", json!({})).await;
    assert_eq!(json_of(&body), Value::Null);
}

#[tokio::test]
async fn errors_come_back_as_a_message() {
    let server = server();
    let (status, _, body) = call(&server, "create_topic", json!({ "name": "  ", "module": null })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json_of(&body)["message"].as_str().unwrap().len() > 3);
    let (status, _, body) = call(&server, "topic_cards", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json_of(&body)["message"].as_str().unwrap().starts_with("the topic argument is missing"));
}

#[tokio::test]
async fn images_go_in_as_base64_and_come_back_as_bytes() {
    let server = server();
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0];
    use base64::Engine;
    let (status, _, body) = call(&server, "add_image", json!({ "data": base64::engine::general_purpose::STANDARD.encode(png) })).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, kind, body) = call(&server, "image", json!({ "id": json_of(&body) })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(kind, "application/octet-stream");
    assert_eq!(body, png);
}

#[tokio::test]
async fn serves_the_frontend_for_any_other_path() {
    let server = server();
    let response = server.oneshot(Request::get("/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("Study App"));
}

#[test]
fn dispatch_replies_with_bytes_only_for_images() {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    assert!(matches!(dispatch(&mut app, "modules", &json!({}), TODAY), Ok(Reply::Json(_))));
}

// ---------- Hazards 12 and 13: only this app's own page may call the server ----------

/// A request as a browser or another program would send it, with whichever headers.
async fn raw(server: &axum::Router, command: &str, headers: &[(&str, &str)], body: &str) -> (StatusCode, String) {
    let mut request = Request::post(format!("/api/{command}"));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = server.clone().oneshot(request.body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    (status, String::from_utf8_lossy(&response.into_body().collect().await.unwrap().to_bytes()).to_string())
}

async fn module_names(server: &axum::Router) -> Vec<String> {
    let (_, _, body) = call(server, "modules", json!({})).await;
    json_of(&body).as_array().unwrap().iter().map(|module| module["name"].as_str().unwrap().to_string()).collect()
}

// There are no logins, so the server trusts whoever can reach it. A web page open in the
// same browser can reach 127.0.0.1 too, and used to be able to change or delete anything.
#[tokio::test]
async fn another_website_cannot_call_the_server() {
    let server = server();
    let attack = r#"{"name":"made by another site"}"#;
    let json = ("content-type", "application/json");
    let here = ("host", "127.0.0.1:7392");
    let attempts: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("a form or fetch without preflight (text/plain)", vec![here, ("content-type", "text/plain"), ("origin", "https://evil.example")]),
        ("a form post", vec![here, ("content-type", "application/x-www-form-urlencoded"), ("origin", "https://evil.example")]),
        ("no content type at all", vec![here, ("origin", "https://evil.example")]),
        ("another origin, even with JSON", vec![here, json, ("origin", "https://evil.example")]),
        ("another port on this computer", vec![here, json, ("origin", "http://127.0.0.1:9999")]),
        ("a sandboxed page (null origin)", vec![here, json, ("origin", "null")]),
        ("the browser says it's cross-site", vec![here, json, ("sec-fetch-site", "cross-site")]),
        ("the browser says it's same-site but another origin", vec![here, json, ("sec-fetch-site", "same-site")]),
        ("a name that only resolves here (DNS rebinding)", vec![("host", "evil.example:7392"), json, ("origin", "http://evil.example:7392")]),
        ("a rebinding name without an origin", vec![("host", "evil.example"), json]),
    ];
    for (what, headers) in attempts {
        let (status, body) = raw(&server, "create_module", &headers, attack).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{what}: {body}");
        let (status, body) = raw(&server, "modules", &headers, "{}").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{what} can't read either: {body}");
    }
    assert_eq!(module_names(&server).await, Vec::<String>::new(), "nothing got through");
}

#[tokio::test]
async fn the_apps_own_page_and_local_tools_still_can() {
    let server = server();
    let json = ("content-type", "application/json");
    let ways: Vec<Vec<(&str, &str)>> = vec![
        vec![("host", "127.0.0.1:7392"), json, ("origin", "http://127.0.0.1:7392"), ("sec-fetch-site", "same-origin")],
        vec![("host", "localhost:7392"), json, ("origin", "http://localhost:7392")],
        vec![("host", "[::1]:7392"), json, ("origin", "http://[::1]:7392")],
        vec![("host", "localhost:7400"), ("content-type", "application/json; charset=utf-8")], // curl or a script
        vec![json],
    ];
    for (n, headers) in ways.iter().enumerate() {
        let (status, body) = raw(&server, "create_module", headers, &format!(r#"{{"name":"m{n}"}}"#)).await;
        assert_eq!(status, StatusCode::OK, "{headers:?}: {body}");
    }
    assert_eq!(module_names(&server).await.len(), ways.len());
}

#[tokio::test]
async fn a_host_name_you_allow_works_too() {
    let server = study_web::router_for(App::with_seed(Store::in_memory().unwrap(), 1), &dist(), || TODAY, &["study.lan".to_string()]);
    let json = ("content-type", "application/json");
    let (status, _) = raw(&server, "modules", &[("host", "study.lan:7392"), json, ("origin", "http://study.lan:7392")], "{}").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = raw(&server, "modules", &[("host", "other.lan:7392"), json], "{}").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[test]
fn the_host_setting_is_a_comma_list_with_blanks_ignored() {
    assert_eq!(study_web::host_list(""), Vec::<String>::new());
    assert_eq!(study_web::host_list(" , ,"), Vec::<String>::new());
    assert_eq!(study_web::host_list("Study.LAN, nas.local ,,x"), ["study.lan", "nas.local", "x"]);
}

// The desktop app has a content security policy. The browser version sent none, so
// anything that slipped past the sanitiser could run, and images in imported text could
// phone home.
#[tokio::test]
async fn every_response_carries_the_security_headers() {
    let server = server();
    for request in [Request::get("/").body(Body::empty()).unwrap(), Request::get("/assets/missing.js").body(Body::empty()).unwrap(), Request::post("/api/modules").header("content-type", "application/json").body(Body::from("{}")).unwrap(), Request::post("/api/modules").header("origin", "https://evil.example").body(Body::from("{}")).unwrap()] {
        let path = request.uri().to_string();
        let response = server.clone().oneshot(request).await.unwrap();
        let header = |name: &str| response.headers().get(name).map(|value| value.to_str().unwrap().to_string()).unwrap_or_default();
        let policy = header("content-security-policy");
        for part in ["default-src 'self'", "script-src 'self'", "img-src 'self' data: blob:", "frame-ancestors 'none'", "form-action 'none'", "base-uri 'none'", "object-src 'none'"] {
            assert!(policy.contains(part), "{path}: the policy lacks {part}: {policy}");
        }
        assert!(!policy.contains("unsafe-eval") && !policy.split(';').any(|part| part.trim().starts_with("script-src") && part.contains("unsafe-inline")), "{policy}");
        assert_eq!(header("x-content-type-options"), "nosniff", "{path}");
        assert_eq!(header("x-frame-options"), "DENY", "{path}");
        assert_eq!(header("referrer-policy"), "no-referrer", "{path}");
        assert_eq!(header("cross-origin-resource-policy"), "same-origin", "{path}");
    }
}

// ---------- Hazard 15: the desktop shell's own settings stay locked down ----------

// The desktop shell can't be built in every test environment, but its settings are plain
// JSON. Any script that got into the window could call every command, so the window must
// stay unable to run script that isn't the app's, and hold no permission beyond its own
// commands.
#[test]
fn the_desktop_app_keeps_its_policy_and_asks_for_no_extra_permissions() {
    let read = |path: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(format!("{}/../../src-tauri/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap() };
    let capabilities = read("capabilities/default.json");
    assert_eq!(capabilities["permissions"], json!(["core:default"]), "a new permission is a decision, change this test with it");
    assert_eq!(capabilities["windows"], json!(["main"]));
    assert!(capabilities.get("remote").is_none(), "no website gets to call the app's commands");
    let config = read("tauri.conf.json");
    let security = &config["app"]["security"];
    let policy = security["csp"].as_str().expect("a content security policy");
    for part in ["default-src 'self'", "script-src 'self'", "object-src 'none'", "form-action 'none'", "base-uri 'none'"] {
        assert!(policy.split(';').any(|directive| directive.trim() == part), "the policy lacks {part}: {policy}");
    }
    assert!(!policy.contains("unsafe-eval") && !policy.contains("http:") && !policy.contains("https:") && !policy.contains('*'), "{policy}");
    assert!(!policy.split(';').any(|directive| directive.trim().starts_with("script-src") && directive.contains("unsafe-inline")), "{policy}");
    for switch in ["dangerousDisableAssetCspModification", "freezePrototype", "assetProtocol"] {
        assert!(security.get(switch).is_none() || switch == "freezePrototype", "{switch} is set");
    }
    // The browser version sends the same policy, plus what only a browser needs.
    for directive in policy.split(';').map(str::trim).filter(|directive| !directive.starts_with("img-src")) {
        assert!(study_web::CONTENT_SECURITY_POLICY.contains(directive), "the browser version lacks {directive}");
    }
}
