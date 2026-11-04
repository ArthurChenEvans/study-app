//! PowerPoint to PDF for the source preview (R11), using LibreOffice in headless mode.
//!
//! Slides are untrusted input, so conversion runs with a throwaway LibreOffice profile
//! in a private temp folder that is deleted afterwards, and is killed if it takes too long.
//! Nothing is kept: the PDF bytes go straight back to the preview.

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

pub const MAX_BYTES: usize = 200 * 1024 * 1024;
pub const TIMEOUT: Duration = Duration::from_secs(90);
const SLIDE_EXTENSIONS: &[&str] = &["pptx", "ppt", "odp"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConvertError {
    #[error("only PowerPoint or OpenDocument slides need converting")]
    NotSlides,
    #[error("that file is too big to preview (the limit is 200 MB)")]
    TooLarge,
    #[error("install LibreOffice to open slides, or export them as PDF")]
    LibreOfficeMissing,
    #[error("converting the slides took too long, try exporting them as PDF")]
    TimedOut,
    #[error("LibreOffice couldn't convert these slides: {0}")]
    Failed(String),
}

/// Finds `soffice` or `libreoffice` on the PATH.
pub fn find_libreoffice() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| ["soffice", "libreoffice"].map(|name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

/// A private folder that is always deleted, even if conversion fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> std::io::Result<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("study-app-convert-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn slides_to_pdf(
    bytes: &[u8],
    file_name: &str,
    program: Option<&Path>,
    timeout: Duration,
) -> Result<Vec<u8>, ConvertError> {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .ok_or(ConvertError::NotSlides)?;
    if !SLIDE_EXTENSIONS.contains(&extension.as_str()) {
        return Err(ConvertError::NotSlides);
    }
    if bytes.len() > MAX_BYTES {
        return Err(ConvertError::TooLarge);
    }
    let program = match program {
        Some(path) if path.is_file() => path.to_path_buf(),
        Some(_) => return Err(ConvertError::LibreOfficeMissing),
        None => find_libreoffice().ok_or(ConvertError::LibreOfficeMissing)?,
    };

    let failed = |error: std::io::Error| ConvertError::Failed(error.to_string());
    let scratch = Scratch::new().map_err(failed)?;
    // Never trust the given name for paths: use a fixed one with the checked extension.
    let input = scratch.0.join(format!("slides.{extension}"));
    std::fs::write(&input, bytes).map_err(failed)?;
    let profile = scratch.0.join("profile");

    let mut child = std::process::Command::new(&program)
        .arg(format!(
            "-env:UserInstallation=file://{}",
            profile.display()
        ))
        .args([
            "--headless",
            "--norestore",
            "--convert-to",
            "pdf",
            "--outdir",
        ])
        .arg(&scratch.0)
        .arg(&input)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(failed)?;

    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(failed)? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ConvertError::TimedOut);
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let output = scratch.0.join("slides.pdf");
    match std::fs::read(&output) {
        Ok(pdf) if status.success() && pdf.starts_with(b"%PDF") => Ok(pdf),
        _ => {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                use std::io::Read;
                let _ = pipe.read_to_string(&mut stderr);
            }
            let reason = stderr
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("the file may be damaged or not a presentation");
            Err(ConvertError::Failed(reason.trim().to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conversions share the temp folder, so tests that convert take turns.
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn turn() -> std::sync::MutexGuard<'static, ()> {
        ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn only_slides_are_converted() {
        assert_eq!(
            slides_to_pdf(b"text", "notes.txt", None, TIMEOUT),
            Err(ConvertError::NotSlides)
        );
        assert_eq!(
            slides_to_pdf(b"text", "no-extension", None, TIMEOUT),
            Err(ConvertError::NotSlides)
        );
    }

    #[test]
    fn huge_files_are_refused_before_converting() {
        let huge = vec![0u8; MAX_BYTES + 1];
        assert_eq!(
            slides_to_pdf(&huge, "big.pptx", None, TIMEOUT),
            Err(ConvertError::TooLarge)
        );
    }

    #[test]
    fn says_so_when_libreoffice_is_missing() {
        let missing = Path::new("/nonexistent/soffice");
        assert_eq!(
            slides_to_pdf(b"x", "deck.pptx", Some(missing), TIMEOUT),
            Err(ConvertError::LibreOfficeMissing)
        );
    }

    #[test]
    fn converts_real_slides_to_pdf() {
        let _turn = turn();
        let Some(program) = find_libreoffice() else {
            eprintln!("skipped: LibreOffice is not installed");
            return;
        };
        let pdf = slides_to_pdf(
            &fixture("pipeline.pptx"),
            "Lecture 3.pptx",
            Some(&program),
            TIMEOUT,
        )
        .unwrap();
        assert!(pdf.starts_with(b"%PDF"), "output is a PDF");
    }

    #[test]
    fn broken_slides_fail_with_a_message() {
        let _turn = turn();
        let Some(program) = find_libreoffice() else {
            return;
        };
        let result = slides_to_pdf(
            &fixture("not-slides.txt"),
            "broken.pptx",
            Some(&program),
            TIMEOUT,
        );
        assert!(matches!(result, Err(ConvertError::Failed(_))), "{result:?}");
    }

    #[cfg(unix)]
    #[test]
    fn a_stuck_conversion_is_killed() {
        let _turn = turn();
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("study-convert-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("soffice");
        std::fs::write(&fake, "#!/bin/sh\nsleep 30\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let started = std::time::Instant::now();
        assert_eq!(
            slides_to_pdf(b"x", "deck.pptx", Some(&fake), Duration::from_millis(500)),
            Err(ConvertError::TimedOut)
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leaves_no_files_behind() {
        let _turn = turn();
        let Some(program) = find_libreoffice() else {
            return;
        };
        let before: Vec<_> = leftover_dirs();
        slides_to_pdf(
            &fixture("pipeline.pptx"),
            "deck.pptx",
            Some(&program),
            TIMEOUT,
        )
        .unwrap();
        assert_eq!(leftover_dirs(), before);
    }

    fn leftover_dirs() -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("study-app-convert-"))
            })
            .collect();
        dirs.sort();
        dirs
    }
}
