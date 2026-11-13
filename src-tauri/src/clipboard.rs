//! questions.feature and flashcards.feature (R7): pasting an image. WebKitGTK gives the page an
//! empty clipboard when an image is pasted, so the app reads the system clipboard itself: a
//! copied image (a screenshot, or "Copy image" in a browser) or copied image files (Ctrl+C in
//! a file manager).

use std::path::PathBuf;

const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

/// What's on the clipboard, as far as images go. The real one is the system clipboard.
pub trait Source {
    fn files(&mut self) -> Vec<PathBuf>;
    /// Width, height and RGBA pixels.
    fn image(&mut self) -> Option<(usize, usize, Vec<u8>)>;
}

/// The contents of the image files among `paths` (by extension), skipping anything else.
/// Also used for files dropped on the window.
pub fn image_files(paths: impl IntoIterator<Item = PathBuf>) -> Vec<Vec<u8>> {
    paths
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())))
        .filter_map(|path| std::fs::read(path).ok())
        .collect()
}

/// The image files on the clipboard, or else the copied image as a PNG. Empty if neither.
pub fn images(source: &mut impl Source) -> Vec<Vec<u8>> {
    let files = image_files(source.files());
    if !files.is_empty() {
        return files;
    }
    source.image().and_then(|(width, height, rgba)| png(width, height, &rgba)).into_iter().collect()
}

fn png(width: usize, height: usize, rgba: &[u8]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() != width * height * 4 {
        return None;
    }
    let mut out = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut out, width as u32, height as u32);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;
    Some(out)
}

/// The system clipboard.
pub struct System(arboard::Clipboard);

impl System {
    pub fn open() -> Option<Self> {
        arboard::Clipboard::new().ok().map(System)
    }
}

impl Source for System {
    fn files(&mut self) -> Vec<PathBuf> {
        self.0.get().file_list().unwrap_or_default()
    }
    fn image(&mut self) -> Option<(usize, usize, Vec<u8>)> {
        self.0.get_image().ok().map(|image| (image.width, image.height, image.bytes.into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        files: Vec<PathBuf>,
        image: Option<(usize, usize, Vec<u8>)>,
    }
    impl Source for Fake {
        fn files(&mut self) -> Vec<PathBuf> {
            self.files.clone()
        }
        fn image(&mut self) -> Option<(usize, usize, Vec<u8>)> {
            self.image.clone()
        }
    }

    fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("study-clipboard-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn a_copied_image_becomes_a_png() {
        let red = [200u8, 60, 20, 255].repeat(3 * 2);
        let found = images(&mut Fake { files: vec![], image: Some((3, 2, red)) });
        assert_eq!(found.len(), 1);
        let decoder = ::png::Decoder::new(std::io::Cursor::new(&found[0]));
        let reader = decoder.read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (3, 2));
    }

    #[test]
    fn copied_image_files_are_read_as_they_are_and_other_files_ignored() {
        let png = scratch("diagram.PNG", b"png bytes");
        let notes = scratch("notes.txt", b"words");
        let found = images(&mut Fake { files: vec![notes, png], image: None });
        assert_eq!(found, vec![b"png bytes".to_vec()]);
    }

    #[test]
    fn files_that_arent_images_fall_back_to_a_copied_image() {
        let notes = scratch("only-notes.txt", b"words");
        let found = images(&mut Fake { files: vec![notes], image: Some((1, 1, vec![0, 0, 0, 255])) });
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with(b"\x89PNG"));
    }

    #[test]
    fn dropped_files_keep_only_the_images() {
        let photo = scratch("photo.jpeg", b"jpeg bytes");
        let notes = scratch("dropped.txt", b"words");
        assert_eq!(image_files([notes, photo, PathBuf::from("/nowhere/gone.png")]), vec![b"jpeg bytes".to_vec()]);
    }

    #[test]
    fn nothing_usable_is_nothing() {
        assert!(images(&mut Fake { files: vec![], image: None }).is_empty());
        assert!(images(&mut Fake { files: vec![], image: Some((2, 2, vec![0; 3])) }).is_empty(), "pixels that don't add up");
    }
}
