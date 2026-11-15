//! The phone version (R211): the study core in a browser, run in a Web Worker.
//!
//! The database lives in memory and is kept in the browser between visits as bytes (see
//! `Store::to_bytes`). Sync goes to the account through Supabase with the same code as the
//! desktop app: requests are made with synchronous XHR, which a worker may use freely.

use serde_json::{Value, json};
// Linked for its browser host functions.
#[cfg(target_arch = "wasm32")]
use sqlite_wasm_rs as _;
use study_api::cloud::{Cloud, Http, PROJECT_URL, PUBLISHABLE_KEY, Request, Response};
use study_api::{App, practice::Clock};
use wasm_bindgen::prelude::*;

#[path = "../../study-web/src/dispatch.rs"]
#[allow(dead_code)]
mod dispatch;

/// Now, in the browser's own time zone.
pub fn clock() -> Clock {
    let now = js_sys::Date::new_0();
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date");
    let day = chrono::NaiveDate::from_ymd_opt(now.get_full_year() as i32, now.get_month() + 1, now.get_date()).map_or(0, |date| (date - epoch).num_days());
    Clock { day, minute: now.get_hours() * 60 + now.get_minutes(), unix: (now.get_time() / 1000.0) as i64 }
}

/// Supabase through synchronous XHR (fine in a worker, where nothing waits on it).
struct Xhr;

impl Http for Xhr {
    fn send(&self, request: &Request) -> Result<Response, String> {
        let offline = || "couldn't reach Supabase. Check you're online".to_string();
        let xhr = web_sys::XmlHttpRequest::new().map_err(|_| offline())?;
        xhr.open_with_async(request.method, &request.url, false).map_err(|_| offline())?;
        xhr.set_request_header("Content-Type", "application/json").map_err(|_| offline())?;
        for (name, value) in &request.headers {
            xhr.set_request_header(name, value).map_err(|_| offline())?;
        }
        match &request.body {
            Some(body) => xhr.send_with_opt_str(Some(body)),
            None => xhr.send(),
        }
        .map_err(|_| offline())?;
        let status = xhr.status().map_err(|_| offline())?;
        if status == 0 {
            return Err(offline());
        }
        Ok(Response { status, body: xhr.response_text().ok().flatten().unwrap_or_default() })
    }
}

fn failure(message: impl ToString) -> JsValue {
    let error = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&error, &"message".into(), &message.to_string().into());
    error.into()
}

#[wasm_bindgen]
pub struct Phone {
    app: App,
}

#[wasm_bindgen]
impl Phone {
    /// Opens the kept database (or a new one) and the kept sign-in.
    #[wasm_bindgen(constructor)]
    pub fn new(database: Option<Vec<u8>>, sign_in: Option<String>) -> Result<Phone, JsValue> {
        let store = match database.filter(|bytes| !bytes.is_empty()).map(|bytes| study_store::Store::from_bytes(&bytes)) {
            Some(Ok(store)) => store,
            // A kept copy that can't be read is set aside: the account has everything.
            _ => study_store::Store::in_memory().map_err(failure)?,
        };
        let mut app = App::new(store);
        app.set_sync_default_name("Phone");
        app.set_cloud(Cloud::with(PROJECT_URL, PUBLISHABLE_KEY, Box::new(Xhr)));
        if let Some(text) = sign_in {
            app.cloud_load_text(&text);
        }
        Ok(Phone { app })
    }

    /// Runs one command, the same ones the desktop app has. JSON comes back as a value,
    /// images as bytes.
    pub fn call(&mut self, command: &str, args: &str) -> Result<JsValue, JsValue> {
        let args: Value = serde_json::from_str(args).unwrap_or(json!({}));
        let now = clock();
        match dispatch::dispatch_at(&mut self.app, command, &args, now.day, now) {
            // Plain objects, as the app gets from the desktop (not JavaScript Maps).
            Ok(dispatch::Reply::Json(value)) => serde::Serialize::serialize(&value, &serde_wasm_bindgen::Serializer::json_compatible()).map_err(failure),
            Ok(dispatch::Reply::Bytes(bytes)) => Ok(js_sys::Uint8Array::from(bytes.as_slice()).into()),
            Err(error) => Err(failure(error.message)),
        }
    }

    /// The sign-in, for the browser to keep.
    pub fn sign_in(&self) -> String {
        self.app.cloud_saved_text()
    }

    /// The database, for the browser to keep.
    pub fn database(&mut self) -> Result<Vec<u8>, JsValue> {
        self.app.database_bytes().map_err(|error| failure(error.message))
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn runs_commands_and_keeps_the_database() {
        let mut phone = super::Phone::new(None, None).unwrap();
        phone.call("create_topic", r#"{"name":"DNS","module":null}"#).unwrap();
        let bytes = phone.database().unwrap();
        let mut again = super::Phone::new(Some(bytes), None).unwrap();
        let topics = again.call("topics", "{}").unwrap();
        assert!(js_sys::JSON::stringify(&topics).unwrap().as_string().unwrap().contains("DNS"));
        let signed_out = again.call("cloud_status", "{}").unwrap();
        assert!(js_sys::JSON::stringify(&signed_out).unwrap().as_string().unwrap().contains("\"signed_in\":false"));
    }
}
