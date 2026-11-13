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

/// Finds LibreOffice: `soffice` or `libreoffice` on the PATH, and on Windows `soffice.exe`
/// on the PATH or in its usual Program Files folder (R164).
pub fn find_libreoffice() -> Option<PathBuf> {
    let path = std::env::var_os("PATH");
    let program_files = ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect::<Vec<_>>();
    let mut candidates = libreoffice_candidates(path.as_deref(), cfg!(windows), None);
    for folder in &program_files {
        candidates.extend(libreoffice_candidates(None, cfg!(windows), Some(folder)));
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Where LibreOffice might be, given a PATH value and, on Windows, a Program Files folder.
fn libreoffice_candidates(path: Option<&std::ffi::OsStr>, windows: bool, program_files: Option<&Path>) -> Vec<PathBuf> {
    let names: &[&str] = if windows { &["soffice.exe"] } else { &["soffice", "libreoffice"] };
    let separator = if windows { ';' } else { ':' };
    let mut found: Vec<PathBuf> = path
        .map(|value| value.to_string_lossy().split(separator).filter(|dir| !dir.is_empty()).flat_map(|dir| names.iter().map(move |name| Path::new(dir).join(name))).collect())
        .unwrap_or_default();
    if windows {
        if let Some(folder) = program_files {
            found.push(folder.join("LibreOffice").join("program").join("soffice.exe"));
        }
    }
    found
}

/// A file URL for a local folder, which LibreOffice wants for its profile. Windows paths
/// get forward slashes and a leading slash, and anything unusual is percent-encoded.
fn file_url(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = if text.starts_with('/') { text } else { format!("/{text}") };
    let mut url = String::from("file://");
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => url.push(byte as char),
            _ => url.push_str(&format!("%{byte:02X}")),
        }
    }
    url
}

/// A private folder that is always deleted, even if conversion fails.
struct Scratch(PathBuf);

impl Scratch {
    /// A new folder only this user can open, under a name nobody can guess. It must not
    /// exist yet: taking over a folder (or a link) someone else put there would hand them
    /// the slides, so that's an error, not a fallback.
    fn new() -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join(format!("study-app-convert-{}-{:032x}", std::process::id(), rand::random::<u128>()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&dir)?;
        Ok(Self(dir))
    }
}

/// Stops the converter and everything it started. LibreOffice's launcher is a shell
/// script that runs the real program as a child, so killing the launcher alone leaves
/// that running. The converter is started in its own process group, and the group goes.
fn stop(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        // SAFETY: kill only sends a signal. The negative ID means the process group, which
        // is this child's own (see process_group(0) where it's started), never ours.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
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

    // What the converter says goes to a file, not a pipe: a pipe holds 64 KB, and a
    // converter that wrote more than that stopped dead until the timeout.
    let said = scratch.0.join("converter-output.txt");
    let mut command = std::process::Command::new(&program);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .arg(format!("-env:UserInstallation={}", file_url(&profile)))
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
        .stderr(std::fs::File::create(&said).map_err(failed)?)
        .spawn()
        .map_err(failed)?;

    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(failed)? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            stop(&mut child);
            return Err(ConvertError::TimedOut);
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    // Anything the converter left running goes too, before its folder is deleted.
    stop(&mut child);
    let output = scratch.0.join("slides.pdf");
    match std::fs::read(&output) {
        Ok(pdf) if status.success() && pdf.starts_with(b"%PDF") => Ok(pdf),
        _ => {
            let stderr = std::fs::read(&said).map(|bytes| String::from_utf8_lossy(&bytes[..bytes.len().min(64 * 1024)]).to_string()).unwrap_or_default();
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

    // packaging (R164): the same lookups work for a Windows install
    #[test]
    fn libreoffice_names_and_places_cover_windows() {
        let unix = libreoffice_candidates(Some(std::ffi::OsStr::new("/usr/bin:/opt/lo/bin")), false, None);
        assert_eq!(unix, vec![PathBuf::from("/usr/bin/soffice"), PathBuf::from("/usr/bin/libreoffice"), PathBuf::from("/opt/lo/bin/soffice"), PathBuf::from("/opt/lo/bin/libreoffice")]);
        let windows = libreoffice_candidates(Some(std::ffi::OsStr::new("C:\\Tools")), true, Some(Path::new("C:\\Program Files")));
        assert!(windows.contains(&PathBuf::from("C:\\Tools").join("soffice.exe")));
        assert!(windows.contains(&Path::new("C:\\Program Files").join("LibreOffice").join("program").join("soffice.exe")));
        assert!(!windows.iter().any(|path| path.extension().is_none()), "Windows only looks for .exe");
    }

    #[test]
    fn the_profile_is_a_proper_file_url() {
        assert_eq!(file_url(Path::new("/tmp/study/profile")), "file:///tmp/study/profile");
        assert_eq!(file_url(Path::new("C:\\Users\\Chen\\AppData\\Local\\Temp\\profile")), "file:///C:/Users/Chen/AppData/Local/Temp/profile");
        assert_eq!(file_url(Path::new("/tmp/with space")), "file:///tmp/with%20space");
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
        // Older LibreOffice refuses a text file named .pptx. Newer ones open it as text and
        // convert that. Either is fine, as long as it's an answer and not a hang or a
        // crash. A converter that fails is covered in audit_tests with a stand-in.
        match result {
            Err(ConvertError::Failed(reason)) => assert!(!reason.is_empty()),
            Ok(pdf) => assert!(pdf.starts_with(b"%PDF")),
            other => panic!("{other:?}"),
        }
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
