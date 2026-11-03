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
