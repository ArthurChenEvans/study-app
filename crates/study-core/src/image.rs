//! Images in questions, options and explanations (R7). The bytes are checked here so
//! only real raster images are kept. SVG is refused on purpose, since it can carry script.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageId(pub i64);

/// Big enough for a full-screen screenshot, small enough to keep the database quick.
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageFormat {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImageError {
    #[error("the image is empty")]
    Empty,
    #[error("the image is over 10 MB, use a smaller one")]
    TooLarge,
    #[error("that file isn't a PNG, JPEG, GIF or WebP image")]
    NotAnImage,
}

/// Works out the format from the file's own bytes, never from its name.
pub fn check_image(bytes: &[u8]) -> Result<ImageFormat, ImageError> {
    if bytes.is_empty() {
        return Err(ImageError::Empty);
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(ImageFormat::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Ok(ImageFormat::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Ok(ImageFormat::Gif)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok(ImageFormat::Webp)
    } else {
        Err(ImageError::NotAnImage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // questions.feature: Add an image by pasting or picking
    #[test]
    fn recognises_each_supported_format_from_its_bytes() {
        assert_eq!(check_image(b"\x89PNG\r\n\x1a\nrest"), Ok(ImageFormat::Png));
        assert_eq!(check_image(&[0xFF, 0xD8, 0xFF, 0xE0, 0]), Ok(ImageFormat::Jpeg));
        assert_eq!(check_image(b"GIF89a...."), Ok(ImageFormat::Gif));
        assert_eq!(check_image(b"RIFF\0\0\0\0WEBPVP8 "), Ok(ImageFormat::Webp));
    }

    #[test]
    fn refuses_svg_and_other_files() {
        assert_eq!(check_image(b"<svg onload=\"alert(1)\"/>"), Err(ImageError::NotAnImage));
        assert_eq!(check_image(b"%PDF-1.7"), Err(ImageError::NotAnImage));
        assert_eq!(check_image(b"RIFF\0\0\0\0WAVE"), Err(ImageError::NotAnImage));
    }

    #[test]
    fn refuses_empty_and_oversized_images() {
        assert_eq!(check_image(&[]), Err(ImageError::Empty));
        let mut huge = b"\x89PNG\r\n\x1a\n".to_vec();
        huge.resize(MAX_IMAGE_BYTES + 1, 0);
        assert_eq!(check_image(&huge), Err(ImageError::TooLarge));
        huge.truncate(MAX_IMAGE_BYTES);
        assert_eq!(check_image(&huge), Ok(ImageFormat::Png));
    }

    #[test]
    fn gives_the_mime_type_for_display() {
        assert_eq!(ImageFormat::Webp.mime(), "image/webp");
    }
}
