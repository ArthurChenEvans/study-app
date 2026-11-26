//! Portable mode (R227): a copy with `portable.txt` next to its .exe keeps everything in a
//! `data` folder beside it (the database, backups, sync and account files, and the WebView's
//! own storage), so the folder can live anywhere, even a USB stick, and deleting it removes
//! the app completely. The installed copy, without the file, uses the usual app data folder.

use std::path::{Path, PathBuf};

/// The file that turns portable mode on. The portable zip ships it.
pub const MARKER: &str = "portable.txt";

/// The data folder for a copy at `exe`, when it's portable.
pub fn data_dir(exe: &Path) -> Option<PathBuf> {
    let folder = exe.parent()?;
    folder.join(MARKER).is_file().then(|| folder.join("data"))
}

/// Portable mode for this copy, worked out from where it's running.
pub fn here() -> Option<PathBuf> {
    data_dir(&std::env::current_exe().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_with_portable_txt_beside_it_keeps_its_data_there() {
        let folder = std::env::temp_dir().join(format!("shudhana-portable-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let exe = folder.join("Shudhana.exe");
        assert_eq!(data_dir(&exe), None, "no portable.txt: the usual app data folder");
        std::fs::write(folder.join(MARKER), "").unwrap();
        assert_eq!(data_dir(&exe), Some(folder.join("data")));
        std::fs::create_dir_all(folder.join("other")).unwrap();
        assert_eq!(data_dir(&folder.join("other").join("Shudhana.exe")), None, "only right next to the .exe");
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
