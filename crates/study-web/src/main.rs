//! `study-web` serves the app in a browser. `study-web seed <database>` fills an empty
//! database with the demo profile (works on the desktop app's database too), and
//! `study-web seed <database> --large` with the heavy one for testing at scale.

use study_api::App;
use study_store::Store;
use study_web::{Config, today};

fn seed(app: &mut App, large: bool) -> Result<(), String> {
    let (summary, days) = if large {
        (study_api::seed_large::seed_large(app, today()), study_api::seed_large::LARGE_DAYS)
    } else {
        (study_api::seed::seed_profile(app, today()), study_api::seed::DAYS)
    };
    let summary = summary.map_err(|error| error.message)?;
    println!(
        "seeded {} modules, {} topics, {} questions, {} sources, {} quizzes, {} recall checks, {} card reviews and {} projects with {} struggles over {} days",
        summary.modules, summary.topics, summary.questions, summary.sources, summary.quizzes, summary.checks, summary.reviews, summary.projects, summary.struggles, days
    );
    Ok(())
}

fn open(path: &std::path::Path) -> Result<App, String> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|error| format!("couldn't make {}: {error}", parent.display()))?;
    }
    Store::open(path).map(App::new).map_err(|error| format!("couldn't open {}: {error}", path.display()))
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("seed") {
        let path = args.get(1).ok_or("usage: study-web seed <path to study.sqlite> [--large]")?;
        return seed(&mut open(path.as_ref())?, args.iter().any(|arg| arg == "--large"));
    }

    let config = Config::from_env();
    // An update that upgrades the database copies it first (R102).
    study_api::backup::before_update(&config.db, &config.backups, study_web::clock()).map_err(|error| error.message)?;
    let mut app = open(&config.db)?;
    app.set_backups(&config.backups);
    // Sync settings live beside the backups, and this version is "Browser" until named.
    app.set_sync_home(config.backups.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(std::path::Path::new(".")));
    app.set_sync_default_name("Browser");
    if config.seed {
        if app.topics().map_err(|error| error.message)?.is_empty() {
            seed(&mut app, config.large)?;
        } else {
            println!("the database already has data, so it wasn't seeded");
        }
    }
    if let Err(error) = app.daily_backup(study_web::clock()) {
        println!("couldn't make today's backup: {}", error.message);
    }
    let listener = tokio::net::TcpListener::bind(&config.address).await.map_err(|error| format!("couldn't listen on {}: {error}", config.address))?;
    println!("Study App on http://{} (database {})", config.address, config.db.display());
    axum::serve(listener, study_web::router_for(app, &config.dist, today, &config.hosts))
        .with_graceful_shutdown(stopped())
        .await
        .map_err(|error| error.to_string())
}

/// Ctrl+C, or SIGTERM from `podman stop` / `compose down`.
async fn stopped() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}
