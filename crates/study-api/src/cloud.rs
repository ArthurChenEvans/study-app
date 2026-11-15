//! Accounts and sync through Supabase (R206 to R210).
//!
//! Signing in is passwordless: Supabase emails a 6-digit code, and typing it in gives this
//! computer a session (a short-lived access token and a refresh token). Two-step sign-in is an
//! authenticator app (TOTP), opt-in. Sync sends the same change lines folder sync writes, to a
//! `changes` table that Row Level Security keeps to the account's own rows (see
//! `supabase/migrations`). Images stay on the computer while the admin has them switched off.
//!
//! Every request goes through [`Http`], so the tests can answer for Supabase.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use study_store::sync::Line;

use crate::practice::Clock;
use crate::{ApiError, ApiResult, App};

/// The project the app syncs with. The publishable key only identifies the project: what an
/// account can see is decided by the database's rules, so it's safe in the app.
pub const PROJECT_URL: &str = "https://cqsxbtmonpxcywvznlao.supabase.co";
pub const PUBLISHABLE_KEY: &str = "sb_publishable_iRXgaLMfgves2nxgbtjBSw__BJuTYlG";

/// Lines sent or read per request, and read per sync.
const BATCH: usize = 500;
const LINES_PER_SYNC: usize = 100_000;

/// The account's tables don't exist: the setup SQL hasn't been run in Supabase.
pub const NOT_SET_UP: &str = "the account's database isn't set up yet, so nothing synced. Run supabase/migrations in the Supabase SQL Editor (see the README)";

pub const IMAGES_OFF: &str = "Images can't be added right now, there isn't room to store them yet";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

pub trait Http: Send {
    fn send(&self, request: &Request) -> Result<Response, String>;
}

/// The real network, on the desktop. The phone version brings its own (see study-phone).
#[cfg(not(target_arch = "wasm32"))]
pub struct Network;

/// No network: for a build that hasn't been given one.
pub struct Offline;

impl Http for Offline {
    fn send(&self, _request: &Request) -> Result<Response, String> {
        Err("there's no connection to Supabase here".into())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Http for Network {
    fn send(&self, request: &Request) -> Result<Response, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .user_agent(concat!("Shudhana/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        let mut headers = request.headers.clone();
        headers.push(("Content-Type".into(), "application/json".into()));
        let result = match request.method {
            "GET" | "DELETE" => {
                let mut builder = if request.method == "GET" { agent.get(&request.url) } else { agent.delete(&request.url) };
                for (name, value) in &headers {
                    builder = builder.header(name, value);
                }
                builder.call()
            }
            method => {
                let mut builder = match method {
                    "PATCH" => agent.patch(&request.url),
                    _ => agent.post(&request.url),
                };
                for (name, value) in &headers {
                    builder = builder.header(name, value);
                }
                builder.send(request.body.clone().unwrap_or_default())
            }
        };
        let mut response = result.map_err(|error| match error {
            ureq::Error::Timeout(_) => "Supabase took too long to answer".to_string(),
            _ => "couldn't reach Supabase. Check you're online".to_string(),
        })?;
        let status = response.status().as_u16();
        let body = response.body_mut().read_to_string().unwrap_or_default();
        Ok(Response { status, body })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Session {
    access_token: String,
    refresh_token: String,
    /// Unix seconds.
    expires_at: i64,
    user_id: String,
    email: String,
}

pub struct Cloud {
    url: String,
    key: String,
    http: Box<dyn Http>,
    home: Option<PathBuf>,
    session: Option<Session>,
    images_allowed: bool,
    admin: bool,
    problem: Option<String>,
    last_synced: Option<i64>,
    names: BTreeMap<String, String>,
}

impl Default for Cloud {
    fn default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let http: Box<dyn Http> = Box::new(Network);
        #[cfg(target_arch = "wasm32")]
        let http: Box<dyn Http> = Box::new(Offline);
        Self { url: PROJECT_URL.into(), key: PUBLISHABLE_KEY.into(), http, home: None, session: None, images_allowed: false, admin: false, problem: None, last_synced: None, names: BTreeMap::new() }
    }
}

impl std::fmt::Debug for Cloud {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Cloud").field("url", &self.url).field("signed_in", &self.session.is_some()).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CloudDevice {
    pub id: String,
    pub name: String,
    pub this_one: bool,
    /// Unix seconds.
    pub last_seen: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CloudStatus {
    pub signed_in: bool,
    pub email: Option<String>,
    /// Two-step sign-in is on for this account.
    pub two_step: bool,
    /// Signed in with the emailed code, and the authenticator code is still to come.
    pub needs_two_step: bool,
    pub images_allowed: bool,
    pub admin: bool,
    pub devices: Vec<CloudDevice>,
    /// Unix seconds.
    pub last_synced: Option<i64>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TwoStepSetup {
    pub factor: String,
    /// An SVG picture of the QR code, as a data: address.
    pub qr: String,
    /// The same secret as text, for typing into the app by hand.
    pub secret: String,
}

/// What Supabase says when something's wrong, in its own words where it has them.
fn message_of(body: &str, fallback: &str) -> String {
    let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let said = ["msg", "message", "error_description", "error"].iter().find_map(|key| parsed.get(*key).and_then(Value::as_str)).unwrap_or("");
    let lower = said.to_lowercase();
    if lower.contains("schema cache") || parsed.get("code").and_then(Value::as_str) == Some("PGRST205") {
        return NOT_SET_UP.into();
    }
    if lower.contains("expired") || lower.contains("invalid") && (lower.contains("otp") || lower.contains("token")) {
        return "that code didn't work or has run out. Ask for a new one".into();
    }
    if lower.contains("rate limit") || lower.contains("security purposes") {
        return "too many codes asked for. Wait a minute and try again".into();
    }
    if lower.contains("two-step") {
        return "this needs your authenticator code first".into();
    }
    if said.is_empty() { fallback.into() } else { said.trim_end_matches('.').to_string() }
}

fn valid_email(email: &str) -> bool {
    let email = email.trim();
    let Some((name, domain)) = email.split_once('@') else { return false };
    !name.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && !email.contains(char::is_whitespace) && email.len() <= 254
}

/// The `aal` claim of an access token (aal1 after the emailed code, aal2 after the
/// authenticator code). Only read, never trusted for anything: the database checks it.
fn assurance(token: &str) -> String {
    use base64::Engine;
    let Some(payload) = token.split('.').nth(1) else { return String::new() };
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')) else { return String::new() };
    serde_json::from_slice::<Value>(&bytes).ok().and_then(|claims| claims.get("aal").and_then(Value::as_str).map(str::to_string)).unwrap_or_default()
}

impl Cloud {
    /// Points at another project and network, for tests.
    pub fn with(url: &str, key: &str, http: Box<dyn Http>) -> Self {
        Self { url: url.trim_end_matches('/').into(), key: key.into(), http, ..Self::default() }
    }

    fn path(&self) -> Option<PathBuf> {
        self.home.as_ref().map(|home| home.join("account.json"))
    }

    fn save(&self) -> ApiResult<()> {
        let Some(path) = self.path() else { return Ok(()) };
        let saved = self.saved_text();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ApiError::new(format!("couldn't save the sign-in: {error}")))?;
        }
        std::fs::write(&path, saved).map_err(|error| ApiError::new(format!("couldn't save the sign-in: {error}")))?;
        // The refresh token is as good as being signed in, so only this user can read it.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// The sign-in as text, for a build without files (the phone keeps it in the browser).
    pub fn saved_text(&self) -> String {
        json!({ "session": self.session, "last_synced": self.last_synced, "names": self.names }).to_string()
    }

    pub fn load_text(&mut self, text: &str) {
        let saved: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        self.session = serde_json::from_value(saved.get("session").cloned().unwrap_or(Value::Null)).ok();
        self.last_synced = saved.get("last_synced").and_then(Value::as_i64);
        self.names = serde_json::from_value(saved.get("names").cloned().unwrap_or(Value::Null)).unwrap_or_default();
    }

    fn load(&mut self) {
        let Some(path) = self.path() else { return };
        let Ok(text) = std::fs::read_to_string(path) else { return };
        let saved: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        self.session = serde_json::from_value(saved.get("session").cloned().unwrap_or(Value::Null)).ok();
        self.last_synced = saved.get("last_synced").and_then(Value::as_i64);
        self.names = serde_json::from_value(saved.get("names").cloned().unwrap_or(Value::Null)).unwrap_or_default();
    }

    fn auth(&self, method: &'static str, path: &str, body: Option<Value>, token: Option<&str>) -> ApiResult<Response> {
        let mut headers = vec![("apikey".to_string(), self.key.clone())];
        if let Some(token) = token {
            headers.push(("Authorization".into(), format!("Bearer {token}")));
        }
        let request = Request { method, url: format!("{}/auth/v1/{path}", self.url), headers, body: body.map(|body| body.to_string()) };
        self.http.send(&request).map_err(ApiError::new)
    }

    fn take_session(&mut self, body: &str, email: &str) -> ApiResult<()> {
        let parsed: Value = serde_json::from_str(body).map_err(|_| ApiError::new("Supabase sent back something unreadable"))?;
        let text = |key: &str| parsed.get(key).and_then(Value::as_str).map(str::to_string);
        let (Some(access_token), Some(refresh_token)) = (text("access_token"), text("refresh_token")) else {
            return Err(ApiError::new("Supabase didn't sign this computer in"));
        };
        let expires_at = parsed.get("expires_at").and_then(Value::as_i64).unwrap_or_else(|| web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64) + parsed.get("expires_in").and_then(Value::as_i64).unwrap_or(3600));
        let user = parsed.get("user").cloned().unwrap_or(Value::Null);
        let user_id = user.get("id").and_then(Value::as_str).map(str::to_string).or_else(|| self.session.as_ref().map(|session| session.user_id.clone())).unwrap_or_default();
        let email = user.get("email").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| email.to_string());
        self.session = Some(Session { access_token, refresh_token, expires_at, user_id, email });
        self.save()
    }

    /// A working access token, refreshed first when it's about to run out.
    fn token(&mut self, now: i64) -> ApiResult<String> {
        let Some(session) = self.session.clone() else { return Err(ApiError::new("sign in to sync first")) };
        if session.expires_at - 60 > now {
            return Ok(session.access_token);
        }
        let response = self.auth("POST", "token?grant_type=refresh_token", Some(json!({ "refresh_token": session.refresh_token })), None)?;
        if response.status == 200 {
            self.take_session(&response.body, &session.email)?;
            return Ok(self.session.as_ref().map(|session| session.access_token.clone()).unwrap_or_default());
        }
        if (400..500).contains(&response.status) {
            // Signed out elsewhere, or the account was deleted.
            self.session = None;
            self.save()?;
            return Err(ApiError::new("you've been signed out. Sign in again to keep syncing"));
        }
        Err(ApiError::new(message_of(&response.body, "couldn't keep you signed in")))
    }

    fn rest(&mut self, method: &'static str, path: &str, body: Option<Value>, prefer: Option<&str>, now: i64) -> ApiResult<Response> {
        let token = self.token(now)?;
        let mut headers = vec![("apikey".to_string(), self.key.clone()), ("Authorization".into(), format!("Bearer {token}"))];
        if let Some(prefer) = prefer {
            headers.push(("Prefer".into(), prefer.into()));
        }
        let request = Request { method, url: format!("{}/rest/v1/{path}", self.url), headers, body: body.map(|body| body.to_string()) };
        let response = self.http.send(&request).map_err(ApiError::new)?;
        if response.status >= 300 {
            return Err(ApiError::new(message_of(&response.body, &format!("Supabase answered with an error ({})", response.status))));
        }
        Ok(response)
    }

    fn factors(&mut self, now: i64) -> ApiResult<Vec<Value>> {
        let token = self.token(now)?;
        let response = self.auth("GET", "user", None, Some(&token))?;
        if response.status != 200 {
            return Err(ApiError::new(message_of(&response.body, "couldn't read your account")));
        }
        let user: Value = serde_json::from_str(&response.body).unwrap_or(Value::Null);
        Ok(user.get("factors").and_then(Value::as_array).cloned().unwrap_or_default())
    }

    fn verified_factor(&mut self, now: i64) -> ApiResult<Option<String>> {
        Ok(self.factors(now)?.into_iter().find(|factor| factor.get("status").and_then(Value::as_str) == Some("verified")).and_then(|factor| factor.get("id").and_then(Value::as_str).map(str::to_string)))
    }

    fn aal(&self) -> String {
        self.session.as_ref().map(|session| assurance(&session.access_token)).unwrap_or_default()
    }

    pub fn signed_in(&self) -> bool {
        self.session.is_some()
    }

    /// Whether images can be added: always without an account, and with one while the admin allows it.
    pub fn images_allowed(&self) -> bool {
        self.session.is_none() || self.images_allowed
    }
}

impl App {
    /// Where the sign-in is kept. The shells set this when the app starts, beside the sync settings.
    pub fn set_cloud_home(&mut self, dir: impl Into<PathBuf>) {
        self.cloud.home = Some(dir.into());
        self.cloud.load();
    }

    /// Swaps in another project and network, for tests.
    pub fn set_cloud(&mut self, cloud: Cloud) {
        let home = self.cloud.home.take();
        self.cloud = cloud;
        if let Some(home) = home {
            self.set_cloud_home(home);
        }
    }

    pub fn images_allowed(&self) -> bool {
        self.cloud.images_allowed()
    }

    /// The phone keeps the sign-in in the browser: it reads it out after each command...
    pub fn cloud_saved_text(&self) -> String {
        self.cloud.saved_text()
    }

    /// ...and hands it back when the app opens.
    pub fn cloud_load_text(&mut self, text: &str) {
        self.cloud.load_text(text);
    }

    /// Emails a 6-digit sign-in code. A new email makes an account (open sign-up).
    pub fn cloud_request_code(&mut self, email: &str) -> ApiResult<()> {
        let email = email.trim();
        if !valid_email(email) {
            return Err(ApiError::new("that doesn't look like an email address"));
        }
        let response = self.cloud.auth("POST", "otp", Some(json!({ "email": email, "create_user": true })), None)?;
        if response.status >= 300 {
            return Err(ApiError::new(message_of(&response.body, "couldn't send the code")));
        }
        Ok(())
    }

    /// Signs this computer in with the emailed code.
    pub fn cloud_verify_code(&mut self, email: &str, code: &str, clock: Clock) -> ApiResult<CloudStatus> {
        // Supabase sends 6 to 10 digits, depending on the project's "Email OTP Length".
        let code: String = code.chars().filter(char::is_ascii_digit).collect();
        if !(6..=10).contains(&code.len()) {
            return Err(ApiError::new("type the whole code from the email"));
        }
        let response = self.cloud.auth("POST", "verify", Some(json!({ "type": "email", "email": email.trim(), "token": code })), None)?;
        if response.status != 200 {
            return Err(ApiError::new(message_of(&response.body, "that code didn't work")));
        }
        self.cloud.take_session(&response.body, email.trim())?;
        self.cloud.problem = None;
        self.cloud_refresh_settings(clock);
        self.cloud_status(clock)
    }

    /// The authenticator code, for an account with two-step sign-in on.
    pub fn cloud_verify_two_step(&mut self, code: &str, clock: Clock) -> ApiResult<CloudStatus> {
        let Some(factor) = self.cloud.verified_factor(clock.unix)? else { return self.cloud_status(clock) };
        self.cloud_factor_verify(&factor, code, clock)?;
        self.cloud_refresh_settings(clock);
        self.cloud_status(clock)
    }

    fn cloud_factor_verify(&mut self, factor: &str, code: &str, clock: Clock) -> ApiResult<()> {
        let code: String = code.chars().filter(char::is_ascii_digit).collect();
        if code.len() != 6 {
            return Err(ApiError::new("type the 6-digit code from your authenticator app"));
        }
        let token = self.cloud.token(clock.unix)?;
        let challenge = self.cloud.auth("POST", &format!("factors/{factor}/challenge"), Some(json!({})), Some(&token))?;
        if challenge.status != 200 {
            return Err(ApiError::new(message_of(&challenge.body, "couldn't start the check")));
        }
        let challenge: Value = serde_json::from_str(&challenge.body).unwrap_or(Value::Null);
        let challenge_id = challenge.get("id").and_then(Value::as_str).unwrap_or("");
        let verified = self.cloud.auth("POST", &format!("factors/{factor}/verify"), Some(json!({ "challenge_id": challenge_id, "code": code })), Some(&token))?;
        if verified.status != 200 {
            return Err(ApiError::new("that code didn't work. Codes change every 30 seconds, so use the newest one"));
        }
        let email = self.cloud.session.as_ref().map(|session| session.email.clone()).unwrap_or_default();
        self.cloud.take_session(&verified.body, &email)
    }

    /// Starts turning on two-step sign-in: a QR code for the authenticator app.
    pub fn cloud_two_step_start(&mut self, clock: Clock) -> ApiResult<TwoStepSetup> {
        let token = self.cloud.token(clock.unix)?;
        // A setup that was never finished is cleared first, or Supabase refuses a second.
        for factor in self.cloud.factors(clock.unix)? {
            if factor.get("status").and_then(Value::as_str) == Some("unverified")
                && let Some(id) = factor.get("id").and_then(Value::as_str)
            {
                self.cloud.auth("DELETE", &format!("factors/{id}"), None, Some(&token))?;
            }
        }
        let response = self.cloud.auth("POST", "factors", Some(json!({ "factor_type": "totp", "friendly_name": "Authenticator app", "issuer": "Shudhana" })), Some(&token))?;
        if response.status != 200 {
            return Err(ApiError::new(message_of(&response.body, "couldn't start two-step sign-in")));
        }
        let made: Value = serde_json::from_str(&response.body).unwrap_or(Value::Null);
        let text = |path: &[&str]| path.iter().try_fold(&made, |value, key| value.get(*key)).and_then(Value::as_str).unwrap_or("").to_string();
        Ok(TwoStepSetup { factor: text(&["id"]), qr: text(&["totp", "qr_code"]), secret: text(&["totp", "secret"]) })
    }

    /// Finishes turning on two-step sign-in with a first code from the app.
    pub fn cloud_two_step_confirm(&mut self, factor: &str, code: &str, clock: Clock) -> ApiResult<CloudStatus> {
        self.cloud_factor_verify(factor, code, clock)?;
        self.cloud_status(clock)
    }

    /// Turns two-step sign-in off. Needs the authenticator code first, like anything else.
    pub fn cloud_two_step_off(&mut self, clock: Clock) -> ApiResult<CloudStatus> {
        if self.cloud.aal() != "aal2" {
            return Err(ApiError::new("this needs your authenticator code first"));
        }
        let token = self.cloud.token(clock.unix)?;
        for factor in self.cloud.factors(clock.unix)? {
            if let Some(id) = factor.get("id").and_then(Value::as_str) {
                let response = self.cloud.auth("DELETE", &format!("factors/{id}"), None, Some(&token))?;
                if response.status >= 300 {
                    return Err(ApiError::new(message_of(&response.body, "couldn't turn two-step sign-in off")));
                }
            }
        }
        self.cloud_status(clock)
    }

    /// Signs this computer out. What's here stays: the data is still the user's own.
    pub fn cloud_sign_out(&mut self, clock: Clock) -> ApiResult<CloudStatus> {
        if let Some(session) = self.cloud.session.clone() {
            // Best effort: the session is forgotten here whether or not Supabase hears about it.
            let _ = self.cloud.auth("POST", "logout?scope=local", None, Some(&session.access_token));
        }
        self.cloud.session = None;
        self.cloud.images_allowed = false;
        self.cloud.admin = false;
        self.cloud.save()?;
        self.cloud_status(clock)
    }

    /// Signs every other computer out of this account.
    pub fn cloud_sign_out_others(&mut self, clock: Clock) -> ApiResult<CloudStatus> {
        let token = self.cloud.token(clock.unix)?;
        let response = self.cloud.auth("POST", "logout?scope=others", None, Some(&token))?;
        if response.status >= 300 {
            return Err(ApiError::new(message_of(&response.body, "couldn't sign the other computers out")));
        }
        // The other computers' rows stay: a computer signing in later reads their changes through them.
        self.cloud_status(clock)
    }

    /// Deletes the account and everything synced to it (R209). What's on this computer stays.
    pub fn cloud_delete_account(&mut self, clock: Clock) -> ApiResult<CloudStatus> {
        self.cloud.rest("POST", "rpc/delete_my_account", Some(json!({})), None, clock.unix)?;
        self.cloud.session = None;
        self.cloud.images_allowed = false;
        self.cloud.admin = false;
        self.cloud.last_synced = None;
        self.cloud.save()?;
        self.cloud_status(clock)
    }

    /// Lets everyone add images, or stops them (admin only, R210).
    pub fn cloud_set_images(&mut self, allowed: bool, clock: Clock) -> ApiResult<CloudStatus> {
        if !self.cloud.admin {
            return Err(ApiError::new("only the admin can change that"));
        }
        let response = self.cloud.rest("PATCH", "app_settings?id=eq.1", Some(json!({ "images_allowed": allowed })), Some("return=representation"), clock.unix)?;
        // The database's rules leave a non-admin's change with no rows rather than an error.
        let rows: Value = serde_json::from_str(&response.body).unwrap_or(Value::Null);
        if rows.as_array().is_none_or(Vec::is_empty) {
            return Err(ApiError::new("only the admin can change that"));
        }
        self.cloud.images_allowed = allowed;
        self.cloud_status(clock)
    }

    /// The admin switches. Read quietly: when it can't be read, images stay off.
    fn cloud_refresh_settings(&mut self, clock: Clock) {
        if self.cloud.aal() != "aal2" && self.cloud.verified_factor(clock.unix).ok().flatten().is_some() {
            return;
        }
        let settings = self.cloud.rest("GET", "app_settings?select=images_allowed", None, None, clock.unix);
        self.cloud.images_allowed = settings.ok().and_then(|response| serde_json::from_str::<Value>(&response.body).ok()).and_then(|rows| rows.get(0)?.get("images_allowed")?.as_bool()).unwrap_or(false);
        let admins = self.cloud.rest("GET", "admins?select=user_id", None, None, clock.unix);
        self.cloud.admin = admins.ok().and_then(|response| serde_json::from_str::<Value>(&response.body).ok()).is_some_and(|rows| rows.as_array().is_some_and(|rows| !rows.is_empty()));
    }

    pub fn cloud_status(&mut self, clock: Clock) -> ApiResult<CloudStatus> {
        let Some(session) = self.cloud.session.clone() else {
            return Ok(CloudStatus { signed_in: false, email: None, two_step: false, needs_two_step: false, images_allowed: true, admin: false, devices: vec![], last_synced: None, problem: self.cloud.problem.clone() });
        };
        let factor = self.cloud.verified_factor(clock.unix);
        if let Err(error) = &factor
            && self.cloud.session.is_none()
        {
            // Signed out while checking.
            self.cloud.problem = Some(error.message.clone());
            return self.cloud_status(clock);
        }
        let two_step = factor.as_ref().is_ok_and(Option::is_some);
        let needs_two_step = two_step && self.cloud.aal() != "aal2";
        let me = self.store.device_id();
        let devices = if needs_two_step {
            vec![]
        } else {
            let rows = self.cloud.rest("GET", "devices?select=device,name,seen_at&order=seen_at.desc", None, None, clock.unix).ok().and_then(|response| serde_json::from_str::<Vec<Value>>(&response.body).ok()).unwrap_or_default();
            rows.iter()
                .filter_map(|row| {
                    let id = row.get("device")?.as_str()?.to_string();
                    let last_seen = row.get("seen_at").and_then(Value::as_str).and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok()).map(|at| at.timestamp());
                    Some(CloudDevice { this_one: id == me, name: row.get("name").and_then(Value::as_str).unwrap_or("A computer").to_string(), id, last_seen })
                })
                .collect()
        };
        Ok(CloudStatus { signed_in: true, email: Some(session.email), two_step, needs_two_step, images_allowed: self.cloud.images_allowed, admin: self.cloud.admin, devices, last_synced: self.cloud.last_synced, problem: self.cloud.problem.clone() })
    }

    /// Sends this computer's new lines and takes in the other computers' (R207). Called from
    /// `sync_now`, after the lines are collected. Returns (sent, lines read, lines).
    pub(crate) fn cloud_exchange(&mut self, name: &str, clock: Clock) -> ApiResult<(usize, Vec<(String, Line)>)> {
        if !self.cloud.signed_in() {
            return Ok((0, vec![]));
        }
        if self.cloud.aal() != "aal2" && self.cloud.verified_factor(clock.unix)?.is_some() {
            return Ok((0, vec![]));
        }
        self.cloud_refresh_settings(clock);
        let me = self.store.device_id();
        let user = self.cloud.session.as_ref().map(|session| session.user_id.clone()).unwrap_or_default();
        let pushed_key = format!("cloud_pushed:{user}");

        // This computer's lines may already be further on in the account, after a backup is restored.
        let devices = self.cloud.rest("GET", "devices?select=device,name,seq", None, None, clock.unix)?;
        let devices: Vec<Value> = serde_json::from_str(&devices.body).unwrap_or_default();
        let mine = devices.iter().find(|row| row.get("device").and_then(Value::as_str) == Some(me.as_str())).and_then(|row| row.get("seq")?.as_u64()).unwrap_or(0);
        self.store.sync_seq_floor(mine)?;

        let pushed: u64 = self.store.sync_meta(&pushed_key)?.and_then(|seq| seq.parse().ok()).unwrap_or(0).max(mine);
        let lines = self.store.sync_outbox(pushed)?;
        let last = self.store.sync_last_seq()?;
        let images = self.cloud.images_allowed;
        let rows: Vec<Value> = lines
            .iter()
            .filter_map(|text| serde_json::from_str::<Value>(text).ok())
            // Images stay on this computer while they're switched off (R210).
            .filter(|line| images || line.get("kind").and_then(Value::as_str) != Some("images"))
            .map(|line| json!({ "device": me, "seq": line.get("seq").cloned().unwrap_or(Value::Null), "line": line }))
            .collect();
        for batch in rows.chunks(BATCH) {
            self.cloud.rest("POST", "changes?on_conflict=user_id,device,seq", Some(Value::Array(batch.to_vec())), Some("resolution=ignore-duplicates,return=minimal"), clock.unix)?;
        }
        let now = chrono::DateTime::from_timestamp(clock.unix, 0).map(|at| at.to_rfc3339()).unwrap_or_default();
        self.cloud.rest("POST", "devices?on_conflict=user_id,device", Some(json!([{ "device": me, "name": name, "seq": last, "seen_at": now, "app": env!("CARGO_PKG_VERSION") }])), Some("resolution=merge-duplicates,return=minimal"), clock.unix)?;
        self.store.set_sync_meta(&pushed_key, &last.to_string())?;

        let applied = self.store.sync_applied()?;
        let mut incoming = vec![];
        for row in &devices {
            let Some(device) = row.get("device").and_then(Value::as_str) else { continue };
            if device == me {
                continue;
            }
            if let Some(device_name) = row.get("name").and_then(Value::as_str) {
                self.cloud.names.insert(device.to_string(), device_name.to_string());
            }
            let mut after = applied.get(device).copied().unwrap_or(0);
            loop {
                if incoming.len() >= LINES_PER_SYNC {
                    break;
                }
                let page = self.cloud.rest("GET", &format!("changes?select=seq,line&device=eq.{device}&seq=gt.{after}&order=seq.asc&limit={BATCH}"), None, None, clock.unix)?;
                let page: Vec<Value> = serde_json::from_str(&page.body).unwrap_or_default();
                let count = page.len();
                for entry in page {
                    if let Some(line) = entry.get("line").cloned().and_then(|line| serde_json::from_value::<Line>(line).ok()) {
                        after = after.max(line.seq);
                        incoming.push((device.to_string(), line));
                    }
                }
                if count < BATCH {
                    break;
                }
            }
        }
        self.cloud.last_synced = Some(clock.unix);
        self.cloud.problem = None;
        self.cloud.save()?;
        Ok((rows.len(), incoming))
    }

    pub(crate) fn cloud_problem(&mut self, problem: Option<String>) {
        self.cloud.problem = problem;
    }

    pub(crate) fn cloud_name_of(&self, device: &str) -> Option<String> {
        self.cloud.names.get(device).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// (method, path start, status, body)
    type Reply = (&'static str, String, u16, String);

    /// Answers each request with the latest matching reply, and keeps a log.
    #[derive(Clone, Default)]
    struct Fake {
        replies: Arc<Mutex<Vec<Reply>>>,
        log: Arc<Mutex<Vec<Request>>>,
    }

    impl Fake {
        fn reply(&self, method: &'static str, path: &str, status: u16, body: Value) {
            self.replies.lock().unwrap().push((method, path.to_string(), status, body.to_string()));
        }
        fn sent(&self, method: &str, path: &str) -> Vec<Request> {
            self.log.lock().unwrap().iter().filter(|request| request.method == method && request.url.contains(path)).cloned().collect()
        }
    }

    impl Http for Fake {
        fn send(&self, request: &Request) -> Result<Response, String> {
            self.log.lock().unwrap().push(request.clone());
            let path = request.url.split_once(".test").map(|(_, path)| path.to_string()).unwrap_or_default();
            let replies = self.replies.lock().unwrap();
            let found = replies.iter().rev().find(|(method, start, _, _)| *method == request.method && path.starts_with(start.as_str()));
            Ok(found.map(|(_, _, status, body)| Response { status: *status, body: body.clone() }).unwrap_or(Response { status: 404, body: "{}".into() }))
        }
    }

    fn token(aal: &str) -> String {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json!({ "aal": aal, "sub": "user-1" }).to_string());
        format!("head.{payload}.sig")
    }

    fn session(aal: &str) -> Value {
        json!({ "access_token": token(aal), "refresh_token": "refresh-1", "expires_at": 2_000_000_000i64, "user": { "id": "user-1", "email": "chen@example.com" } })
    }

    const CLOCK: Clock = Clock { day: 20_000, minute: 600, unix: 1_800_000_000 };

    fn app_with(fake: &Fake) -> App {
        let mut app = App::new(study_store::Store::in_memory().unwrap());
        app.set_cloud(Cloud::with("https://project.test", "key-1", Box::new(fake.clone())));
        app
    }

    fn signed_in(fake: &Fake) -> App {
        fake.reply("POST", "/auth/v1/verify", 200, session("aal1"));
        fake.reply("GET", "/auth/v1/user", 200, json!({ "factors": [] }));
        fake.reply("GET", "/rest/v1/app_settings", 200, json!([{ "images_allowed": false }]));
        fake.reply("GET", "/rest/v1/admins", 200, json!([]));
        fake.reply("GET", "/rest/v1/devices", 200, json!([]));
        let mut app = app_with(fake);
        app.cloud_verify_code("chen@example.com", "123456", CLOCK).unwrap();
        app
    }

    #[test]
    fn asks_for_a_code_and_signs_in_with_it() {
        let fake = Fake::default();
        fake.reply("POST", "/auth/v1/otp", 200, json!({}));
        let mut app = app_with(&fake);
        assert!(app.cloud_request_code("not an email").is_err());
        app.cloud_request_code(" chen@example.com ").unwrap();
        let otp = &fake.sent("POST", "/auth/v1/otp")[0];
        assert_eq!(serde_json::from_str::<Value>(otp.body.as_ref().unwrap()).unwrap(), json!({ "email": "chen@example.com", "create_user": true }));
        assert!(otp.headers.contains(&("apikey".into(), "key-1".into())));

        let mut app = signed_in(&fake);
        let verify = &fake.sent("POST", "/auth/v1/verify")[0];
        assert_eq!(serde_json::from_str::<Value>(verify.body.as_ref().unwrap()).unwrap(), json!({ "type": "email", "email": "chen@example.com", "token": "123456" }));
        // Supabase's codes are 6 to 10 digits long, depending on the project's setting.
        app.cloud_verify_code("chen@example.com", " 1234 5678 ", CLOCK).unwrap();
        let verify = fake.sent("POST", "/auth/v1/verify").pop().unwrap();
        assert_eq!(serde_json::from_str::<Value>(verify.body.as_ref().unwrap()).unwrap()["token"], json!("12345678"));
        let status = app.cloud_status(CLOCK).unwrap();
        assert!(status.signed_in && !status.two_step && !status.needs_two_step && !status.images_allowed);
        assert_eq!(status.email.as_deref(), Some("chen@example.com"));
        assert!(!app.images_allowed(), "images stay off while the admin has them off");
    }

    #[test]
    fn says_why_a_code_was_refused() {
        let fake = Fake::default();
        fake.reply("POST", "/auth/v1/verify", 403, json!({ "msg": "Token has expired or is invalid" }));
        let mut app = app_with(&fake);
        assert!(app.cloud_verify_code("chen@example.com", "12345", CLOCK).unwrap_err().message.contains("code from the email"));
        assert!(app.cloud_verify_code("chen@example.com", "12345678901", CLOCK).is_err());
        assert_eq!(app.cloud_verify_code("chen@example.com", "123 456", CLOCK).unwrap_err().message, "that code didn't work or has run out. Ask for a new one");
        fake.reply("POST", "/auth/v1/otp", 429, json!({ "msg": "For security purposes, you can only request this after 42 seconds." }));
        assert_eq!(app.cloud_request_code("chen@example.com").unwrap_err().message, "too many codes asked for. Wait a minute and try again");
    }

    #[test]
    fn asks_for_the_authenticator_code_when_two_step_is_on() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        fake.reply("GET", "/auth/v1/user", 200, json!({ "factors": [{ "id": "f1", "status": "verified", "factor_type": "totp" }] }));
        let status = app.cloud_status(CLOCK).unwrap();
        assert!(status.two_step && status.needs_two_step);
        assert_eq!(app.cloud_exchange("Desk", CLOCK).unwrap().0, 0, "nothing syncs before the second step");

        fake.reply("POST", "/auth/v1/factors/f1/challenge", 200, json!({ "id": "c1" }));
        fake.reply("POST", "/auth/v1/factors/f1/verify", 200, session("aal2"));
        let status = app.cloud_verify_two_step("654 321", CLOCK).unwrap();
        assert!(status.two_step && !status.needs_two_step);
        let verify = &fake.sent("POST", "/auth/v1/factors/f1/verify")[0];
        assert_eq!(serde_json::from_str::<Value>(verify.body.as_ref().unwrap()).unwrap(), json!({ "challenge_id": "c1", "code": "654321" }));
    }

    #[test]
    fn turns_two_step_on_with_a_qr_code_and_off_only_after_the_second_step() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        fake.reply("GET", "/auth/v1/user", 200, json!({ "factors": [{ "id": "old", "status": "unverified" }] }));
        fake.reply("DELETE", "/auth/v1/factors/old", 200, json!({}));
        fake.reply("POST", "/auth/v1/factors", 200, json!({ "id": "f2", "totp": { "qr_code": "data:image/svg+xml;utf-8,<svg/>", "secret": "ABC123", "uri": "otpauth://x" } }));
        let setup = app.cloud_two_step_start(CLOCK).unwrap();
        assert_eq!(setup, TwoStepSetup { factor: "f2".into(), qr: "data:image/svg+xml;utf-8,<svg/>".into(), secret: "ABC123".into() });
        assert_eq!(fake.sent("DELETE", "/auth/v1/factors/old").len(), 1, "an unfinished setup is cleared first");

        assert_eq!(app.cloud_two_step_off(CLOCK).unwrap_err().message, "this needs your authenticator code first");
        fake.reply("POST", "/auth/v1/factors/f2/challenge", 200, json!({ "id": "c2" }));
        fake.reply("POST", "/auth/v1/factors/f2/verify", 200, session("aal2"));
        fake.reply("GET", "/auth/v1/user", 200, json!({ "factors": [{ "id": "f2", "status": "verified" }] }));
        assert!(app.cloud_two_step_confirm("f2", "111222", CLOCK).unwrap().two_step);
        fake.reply("DELETE", "/auth/v1/factors/f2", 200, json!({}));
        app.cloud_two_step_off(CLOCK).unwrap();
        assert_eq!(fake.sent("DELETE", "/auth/v1/factors/f2").len(), 1);
    }

    #[test]
    fn sends_new_lines_and_takes_in_other_computers_lines() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        app.create_topic("DNS", None).unwrap();
        app.store.sync_collect().unwrap();
        let me = app.store.device_id();
        fake.reply("GET", "/rest/v1/devices", 200, json!([{ "device": me, "name": "Desk", "seq": 0 }, { "device": "laptop1", "name": "Laptop", "seq": 1 }]));
        fake.reply("POST", "/rest/v1/changes", 201, json!(null));
        fake.reply("POST", "/rest/v1/devices", 201, json!(null));
        let line = json!({ "seq": 1, "at": "0000000001000-laptop1-0", "kind": "topics", "uid": "t-laptop", "set": { "name": "TLS" } });
        fake.reply("GET", "/rest/v1/changes", 200, json!([{ "seq": 1, "line": line }]));

        let (sent, incoming) = app.cloud_exchange("Desk", CLOCK).unwrap();
        assert!(sent >= 1);
        let posted = &fake.sent("POST", "/rest/v1/changes")[0];
        assert!(posted.url.ends_with("on_conflict=user_id,device,seq"));
        assert!(posted.headers.iter().any(|(name, value)| name == "Prefer" && value.contains("ignore-duplicates")));
        assert!(posted.headers.iter().any(|(name, value)| name == "Authorization" && value.starts_with("Bearer head.")));
        let rows: Vec<Value> = serde_json::from_str(posted.body.as_ref().unwrap()).unwrap();
        assert!(rows.iter().all(|row| row["device"] == json!(me)));
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].0, "laptop1");
        let asked = &fake.sent("GET", "/rest/v1/changes")[0];
        assert!(asked.url.contains("device=eq.laptop1&seq=gt.0&order=seq.asc"));

        // Nothing new: nothing sent again.
        fake.log.lock().unwrap().clear();
        app.cloud_exchange("Desk", CLOCK).unwrap();
        assert!(fake.sent("POST", "/rest/v1/changes").is_empty());
    }

    #[test]
    fn keeps_images_on_this_computer_while_they_are_switched_off() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        assert_eq!(app.add_image(&[0x89, b'P', b'N', b'G']).unwrap_err().message, IMAGES_OFF);
        fake.reply("GET", "/rest/v1/devices", 200, json!([]));
        fake.reply("POST", "/rest/v1/changes", 201, json!(null));
        fake.reply("POST", "/rest/v1/devices", 201, json!(null));
        app.cloud_exchange("Desk", CLOCK).unwrap();
        for request in fake.sent("POST", "/rest/v1/changes") {
            assert!(!request.body.unwrap().contains("\"kind\":\"images\""));
        }
    }

    #[test]
    fn only_the_admin_changes_the_image_switch() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        assert_eq!(app.cloud_set_images(true, CLOCK).unwrap_err().message, "only the admin can change that");
        fake.reply("GET", "/rest/v1/admins", 200, json!([{ "user_id": "user-1" }]));
        fake.reply("POST", "/auth/v1/verify", 200, session("aal1"));
        app.cloud_verify_code("chen@example.com", "123456", CLOCK).unwrap();
        fake.reply("PATCH", "/rest/v1/app_settings", 200, json!([{ "images_allowed": true }]));
        assert!(app.cloud_set_images(true, CLOCK).unwrap().images_allowed);
        assert!(app.images_allowed());
    }

    #[test]
    fn refreshes_a_token_that_is_running_out_and_signs_out_when_refused() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        let later = Clock { day: 20_001, minute: 600, unix: 2_000_000_000 };
        fake.reply("POST", "/auth/v1/token", 200, json!({ "access_token": token("aal1"), "refresh_token": "refresh-2", "expires_in": 3600 }));
        app.cloud_status(later).unwrap();
        let refresh = &fake.sent("POST", "/auth/v1/token")[0];
        assert!(refresh.url.ends_with("grant_type=refresh_token"));
        assert_eq!(serde_json::from_str::<Value>(refresh.body.as_ref().unwrap()).unwrap(), json!({ "refresh_token": "refresh-1" }));

        fake.reply("POST", "/auth/v1/token", 400, json!({ "msg": "Invalid Refresh Token" }));
        let much_later = Clock { day: 30_000, minute: 600, unix: 3_000_000_000 };
        let status = app.cloud_status(much_later).unwrap();
        assert!(!status.signed_in);
        assert_eq!(status.problem.as_deref(), Some("you've been signed out. Sign in again to keep syncing"));
    }

    #[test]
    fn says_plainly_when_the_database_was_never_set_up() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        app.create_topic("DNS", None).unwrap();
        fake.reply("GET", "/rest/v1/devices", 404, json!({ "code": "PGRST205", "message": "Could not find the table 'public.devices' in the schema cache" }));
        let said = app.sync_now(false, CLOCK).unwrap_err().message;
        assert_eq!(said, NOT_SET_UP);
        // A sync on its own (every 30 seconds) doesn't shout, but the account shows it.
        app.sync_now(true, CLOCK).unwrap();
        assert_eq!(app.cloud_status(CLOCK).unwrap().problem.as_deref(), Some(NOT_SET_UP));
    }

    #[test]
    fn deletes_the_account_and_signs_out() {
        let fake = Fake::default();
        let mut app = signed_in(&fake);
        fake.reply("POST", "/rest/v1/rpc/delete_my_account", 204, json!(null));
        let status = app.cloud_delete_account(CLOCK).unwrap();
        assert!(!status.signed_in);
        assert_eq!(fake.sent("POST", "/rest/v1/rpc/delete_my_account").len(), 1);
    }

    #[test]
    fn keeps_the_sign_in_private_on_disk() {
        let fake = Fake::default();
        let folder = tempfile::tempdir().unwrap();
        fake.reply("POST", "/auth/v1/verify", 200, session("aal1"));
        fake.reply("GET", "/auth/v1/user", 200, json!({ "factors": [] }));
        let mut app = App::new(study_store::Store::in_memory().unwrap());
        app.set_cloud_home(folder.path());
        app.set_cloud(Cloud::with("https://project.test", "key-1", Box::new(fake.clone())));
        app.cloud_verify_code("chen@example.com", "123456", CLOCK).unwrap();
        let path = folder.path().join("account.json");
        assert!(path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let mut again = App::new(study_store::Store::in_memory().unwrap());
        again.set_cloud(Cloud::with("https://project.test", "key-1", Box::new(fake.clone())));
        again.set_cloud_home(folder.path());
        assert!(again.cloud_status(CLOCK).unwrap().signed_in);
        again.cloud_sign_out(CLOCK).unwrap();
        assert!(!App::new(study_store::Store::in_memory().unwrap()).cloud.signed_in());
        let mut third = App::new(study_store::Store::in_memory().unwrap());
        third.set_cloud_home(folder.path());
        assert!(!third.cloud.signed_in());
    }
}
