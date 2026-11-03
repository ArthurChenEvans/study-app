//! `study-web` serves the app in a browser. `study-web seed <database>` fills an empty
//! database with the demo profile (works on the desktop app's database too).

use study_api::App;
use study_store::Store;
use study_web::{Config, router, today};

fn seed(app: &mut App) -> Result<(), String> {
    let summary = study_api::seed::seed_profile(app, today()).map_err(|error| error.message)?;
    println!(
        "seeded {} modules, {} topics, {} questions, {} sources, {} quizzes, {} recall checks, {} card reviews and {} projects with {} struggles over {} days",
        summary.modules, summary.topics, summary.questions, summary.sources, summary.quizzes, summary.checks, summary.reviews, summary.projects, summary.struggles, study_api::seed::DAYS
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
        let path = args.get(1).ok_or("usage: study-web seed <path to study.sqlite>")?;
        return seed(&mut open(path.as_ref())?);
    }

    let config = Config::from_env();
    let mut app = open(&config.db)?;
    if config.seed {
        if app.topics().map_err(|error| error.message)?.is_empty() {
            seed(&mut app)?;
        } else {
            println!("the database already has data, so it wasn't seeded");
        }
    }
    let listener = tokio::net::TcpListener::bind(&config.address).await.map_err(|error| format!("couldn't listen on {}: {error}", config.address))?;
    println!("Study App on http://{} (database {})", config.address, config.db.display());
    axum::serve(listener, router(app, &config.dist, today))
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
