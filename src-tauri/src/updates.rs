//! updates.feature (R165 to R168): signed updates from published releases.
//!
//! The app asks the release feed (latest.json on the newest GitHub Release) whether there's a
//! newer version than the one installed. The updater plugin only takes a package whose
//! signature matches the public key built into the app, and checks it again before
//! installing. The database is backed up before anything is installed. Installing an .rpm
//! runs `rpm -U` with admin rights, so the system asks for a password (pkexec on Fedora).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

use study_api::{ApiError, ApiResult, App};

/// Tests point the app at a fake feed with this. Normal installs use the configured one.
pub const FEED_OVERRIDE: &str = "STUDY_UPDATE_URL";
/// Checked again on close if the last check was longer ago than this.
const RECHECK_AFTER: Duration = Duration::from_secs(60 * 60);
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Pending>>,
    auto: AtomicBool,
    last_check: Mutex<Option<Instant>>,
    busy: AtomicBool,
    /// Where to look instead of the configured feed (tests, and STUDY_UPDATE_URL).
    feed: Mutex<Option<String>>,
}

impl Updates {
    pub fn with_feed(feed: Option<String>) -> Self {
        Updates { feed: Mutex::new(feed), ..Default::default() }
    }
}

struct Pending {
    update: Update,
    bytes: Option<Vec<u8>>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    pub notes: Option<String>,
    pub date: Option<String>,
    pub ready: bool,
    /// Installing an .rpm needs admin rights, so Linux asks for a password. Windows doesn't.
    pub asks_password: bool,
}

const ASKS_PASSWORD: bool = cfg!(target_os = "linux");

fn failed(error: impl std::fmt::Display) -> ApiError {
    ApiError { message: error.to_string() }
}

fn info(pending: &Pending) -> UpdateInfo {
    UpdateInfo {
        version: pending.update.version.clone(),
        current: pending.update.current_version.clone(),
        notes: pending.update.body.clone().filter(|notes| !notes.trim().is_empty()),
        date: pending.update.date.map(|date| date.to_string()),
        ready: pending.bytes.is_some(),
        asks_password: ASKS_PASSWORD,
    }
}

async fn find<R: Runtime>(app: &AppHandle<R>) -> ApiResult<Option<Update>> {
    let mut builder = app.updater_builder().timeout(CHECK_TIMEOUT);
    let feed = app.state::<Updates>().feed.lock().unwrap().clone();
    if let Some(feed) = feed {
        let url = feed.parse().map_err(|_| failed("the update address isn't a valid URL"))?;
        builder = builder.endpoints(vec![url]).map_err(failed)?;
    }
    builder.build().map_err(failed)?.check().await.map_err(failed)
}

/// Is there a newer release? Remembers it so download and install use the same one.
pub async fn check<R: Runtime>(app: &AppHandle<R>) -> ApiResult<Option<UpdateInfo>> {
    let state = app.state::<Updates>();
    let found = find(app).await;
    *state.last_check.lock().unwrap() = Some(Instant::now());
    let mut pending = state.pending.lock().unwrap();
    match found? {
        None => {
            *pending = None;
            Ok(None)
        }
        Some(update) => {
            // Keep a finished download if it's the same version.
            let keep = pending.as_ref().filter(|old| old.update.version == update.version).and_then(|old| old.bytes.clone());
            let next = Pending { update, bytes: keep };
            let shown = info(&next);
            *pending = Some(next);
            Ok(Some(shown))
        }
    }
}

/// Downloads the pending update. The plugin refuses it if the signature doesn't match.
pub async fn download<R: Runtime>(app: &AppHandle<R>) -> ApiResult<UpdateInfo> {
    let state = app.state::<Updates>();
    let update = state.pending.lock().unwrap().as_ref().map(|pending| pending.update.clone()).ok_or_else(|| failed("there's no update to download"))?;
    let bytes = update.download(|_, _| {}, || {}).await.map_err(failed)?;
    let mut pending = state.pending.lock().unwrap();
    let entry = pending.as_mut().filter(|entry| entry.update.version == update.version).ok_or_else(|| failed("a newer update turned up while downloading, check again"))?;
    entry.bytes = Some(bytes);
    Ok(info(entry))
}

/// Backs up the database, then installs the downloaded update (asking for a password).
pub fn install<R: Runtime>(app: &AppHandle<R>) -> ApiResult<String> {
    let state = app.state::<Updates>();
    let (update, bytes) = {
        let pending = state.pending.lock().unwrap();
        let entry = pending.as_ref().ok_or_else(|| failed("there's no update to install"))?;
        (entry.update.clone(), entry.bytes.clone().ok_or_else(|| failed("the update hasn't downloaded yet"))?)
    };
    {
        let study = app.state::<Mutex<App>>();
        let study = study_api::locked(&study);
        study.back_up_before_update(crate::clock()).map_err(|error| failed(format!("the backup before updating failed, so nothing was installed: {}", error.message)))?;
    }
    update.install(bytes).map_err(failed)?;
    *state.pending.lock().unwrap() = None;
    Ok(update.version)
}

pub fn set_auto<R: Runtime>(app: &AppHandle<R>, on: bool) {
    app.state::<Updates>().auto.store(on, Ordering::SeqCst);
}

/// On close with auto update on: install a ready update, or check first if it's been a while.
/// Returns true when it has taken over closing (it quits the app itself when done).
pub fn on_close<R: Runtime>(app: &AppHandle<R>) -> bool {
    let state = app.state::<Updates>();
    if !state.auto.load(Ordering::SeqCst) || state.busy.swap(true, Ordering::SeqCst) {
        return false;
    }
    let ready = state.pending.lock().unwrap().as_ref().is_some_and(|pending| pending.bytes.is_some());
    let stale = state.last_check.lock().unwrap().is_none_or(|at| at.elapsed() > RECHECK_AFTER);
    if !ready && !stale {
        state.busy.store(false, Ordering::SeqCst);
        return false;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.emit("updating-on-close", ASKS_PASSWORD);
        if !ready {
            // Quietly give up when offline or nothing's new.
            let found = matches!(check(&app).await, Ok(Some(_)));
            if !found || download(&app).await.is_err() {
                app.exit(0);
                return;
            }
        }
        let _ = install(&app);
        app.exit(0);
    });
    true
}

/// `study-app --update-now`: check, download, back up and install with no window, then exit.
/// Exit codes: 0 installed, 3 nothing newer, 4 failed (the reason goes to stderr).
pub fn run_headless<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let code = match check(&app).await {
            Ok(None) => {
                eprintln!("update: nothing newer than {}", app.package_info().version);
                3
            }
            Ok(Some(found)) => match download(&app).await.and_then(|_| install(&app)) {
                Ok(version) => {
                    eprintln!("update: installed {version} (was {})", found.current);
                    0
                }
                Err(error) => {
                    eprintln!("update: {}", error.message);
                    4
                }
            },
            Err(error) => {
                eprintln!("update: {}", error.message);
                4
            }
        };
        app.exit(code);
    });
}

// updates.feature (R165 to R168) against a fake update server on this machine. The fixtures are
// signed with a throwaway test key, never the real one.
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const PUBKEY: &str = include_str!("../tests/fixtures/test.key.pub");
    const SIGNATURE: &str = include_str!("../tests/fixtures/payload.bin.sig");
    const PAYLOAD: &[u8] = include_bytes!("../tests/fixtures/payload.bin");

    /// Serves latest.json and the package until the test ends.
    fn serve(version: &str, package: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let feed = serde_json::json!({
            "version": version,
            "notes": "Week view on phones.",
            "pub_date": "2026-11-14T10:00:00Z",
            "platforms": { tauri_plugin_updater::target().unwrap(): { "signature": SIGNATURE.trim(), "url": format!("{base}/package") } },
        })
        .to_string();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = [0u8; 2048];
                let read = stream.read(&mut request).unwrap_or(0);
                let path = String::from_utf8_lossy(&request[..read]).split_whitespace().nth(1).unwrap_or("").to_string();
                let body: Vec<u8> = if path == "/latest.json" { feed.clone().into_bytes() } else { package.clone() };
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(&body);
            }
        });
        format!("{base}/latest.json")
    }

    fn offline() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        format!("http://{address}/latest.json")
    }

    fn app(feed: String) -> tauri::App<tauri::test::MockRuntime> {
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        // As in tauri.conf.json, but trusting the test key. The fake server is plain http.
        context.config_mut().plugins.0.insert(
            "updater".into(),
            serde_json::json!({ "pubkey": PUBKEY.trim(), "endpoints": [], "dangerousInsecureTransportProtocol": true }),
        );
        tauri::test::mock_builder().plugin(tauri_plugin_updater::Builder::new().build()).manage(Updates::with_feed(Some(feed))).build(context).unwrap()
    }

    fn installed(app: &tauri::App<tauri::test::MockRuntime>) -> String {
        app.package_info().version.to_string()
    }

    #[test]
    fn a_newer_release_is_found_with_its_notes_and_downloads_when_the_signature_matches() {
        let app = app(serve("99.0.0", PAYLOAD.to_vec()));
        let found = tauri::async_runtime::block_on(check(app.handle())).unwrap().expect("a newer release");
        assert_eq!((found.version.as_str(), found.current.as_str(), found.ready), ("99.0.0", installed(&app).as_str(), false));
        assert_eq!(found.notes.as_deref(), Some("Week view on phones."));
        let ready = tauri::async_runtime::block_on(download(app.handle())).unwrap();
        assert!(ready.ready);
    }

    #[test]
    fn the_same_version_is_not_an_update() {
        let probe = app(offline());
        let same = installed(&probe);
        let app = app(serve(&same, PAYLOAD.to_vec()));
        assert_eq!(tauri::async_runtime::block_on(check(app.handle())).unwrap(), None);
    }

    #[test]
    fn an_older_release_is_not_an_update() {
        let app = app(serve("0.0.1", PAYLOAD.to_vec()));
        assert_eq!(tauri::async_runtime::block_on(check(app.handle())).unwrap(), None);
    }

    #[test]
    fn a_package_that_doesnt_match_its_signature_is_refused_and_nothing_is_ready_to_install() {
        let mut tampered = PAYLOAD.to_vec();
        tampered[0] ^= 1;
        let app = app(serve("99.0.0", tampered));
        tauri::async_runtime::block_on(check(app.handle())).unwrap().expect("a newer release");
        let refused = tauri::async_runtime::block_on(download(app.handle())).unwrap_err();
        assert!(refused.message.to_lowercase().contains("signature"), "{}", refused.message);
        let still = tauri::async_runtime::block_on(check(app.handle())).unwrap().unwrap();
        assert!(!still.ready, "a refused download is never kept");
        assert!(install(app.handle()).is_err(), "there's nothing to install");
    }

    #[test]
    fn offline_is_an_error_the_app_can_keep_quiet_about() {
        let app = app(offline());
        assert!(tauri::async_runtime::block_on(check(app.handle())).is_err());
        assert!(!on_close_would_install(app.handle()));
    }

    /// on_close needs an update ready (or a stale check) before it takes over closing.
    fn on_close_would_install<R: Runtime>(app: &AppHandle<R>) -> bool {
        app.state::<Updates>().pending.lock().unwrap().as_ref().is_some_and(|pending| pending.bytes.is_some())
    }
}
