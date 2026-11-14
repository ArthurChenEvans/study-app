//! R201: a web page as a source. The page is fetched once, here, and handed to the reader as
//! HTML; the reader keeps only the readable part and nothing is stored.

use std::io::Read;
use std::time::Duration;
use ureq::ResponseExt;

/// Big enough for any article, small enough not to stall the app on a video.
pub const MAX_PAGE_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FetchedPage {
    /// Where the page ended up, after redirects.
    pub url: String,
    pub html: String,
}

/// The address as typed, made into a web address, or why it can't be one.
pub fn web_address(typed: &str) -> Result<String, String> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Err("type or paste a web address".into());
    }
    let lower = typed.to_ascii_lowercase();
    let full = if lower.starts_with("http://") || lower.starts_with("https://") {
        typed.to_string()
    } else if lower.contains("://") || lower.starts_with("javascript:") || lower.starts_with("file:") || lower.starts_with("data:") {
        return Err("only web pages (http or https) can be opened as a source".into());
    } else {
        format!("https://{typed}")
    };
    let host = full.splitn(4, '/').nth(2).unwrap_or("");
    if host.is_empty() || host.contains(' ') || (!host.contains('.') && !host.starts_with("localhost")) {
        return Err(format!("\"{typed}\" doesn't look like a web address"));
    }
    Ok(full)
}

/// Reads at most `limit` bytes, refusing anything longer.
fn read_capped(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes).map_err(|_| "the page stopped loading part way through".to_string())?;
    if bytes.len() as u64 > limit {
        return Err("that page is too big to open as a source (the limit is 5 MB)".into());
    }
    Ok(bytes)
}

pub fn fetch_page(typed: &str) -> Result<FetchedPage, String> {
    let url = web_address(typed)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent("Shudhana study app (reading a source)")
        .build()
        .into();
    let response = agent.get(&url).call().map_err(|error| match error {
        ureq::Error::StatusCode(404) => "that page wasn't found (404)".to_string(),
        ureq::Error::StatusCode(code) => format!("the site answered with an error ({code})"),
        ureq::Error::Timeout(_) => "the page took too long to load".to_string(),
        _ => "the page couldn't be reached. Check the address and that you're online".to_string(),
    })?;
    let kind = response.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    if !kind.is_empty() && !kind.contains("html") && !kind.contains("text/plain") {
        return Err("that address isn't a web page. If it's a PDF or EPUB, download it and open the file".into());
    }
    let finished = response.get_uri().to_string();
    let bytes = read_capped(response.into_body().into_reader(), MAX_PAGE_BYTES)?;
    Ok(FetchedPage { url: finished, html: String::from_utf8_lossy(&bytes).into_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_a_typed_address_into_a_web_address() {
        assert_eq!(web_address("example.com/post").unwrap(), "https://example.com/post");
        assert_eq!(web_address("  http://example.com ").unwrap(), "http://example.com");
        assert_eq!(web_address("HTTPS://Example.com/a").unwrap(), "HTTPS://Example.com/a");
    }

    #[test]
    fn refuses_what_isnt_a_web_page() {
        for typed in ["", "file:///etc/passwd", "javascript:alert(1)", "ftp://example.com", "data:text/html,hi", "not an address", "words"] {
            assert!(web_address(typed).is_err(), "{typed}");
        }
    }

    #[test]
    fn refuses_a_page_over_the_limit() {
        assert_eq!(read_capped(&b"12345"[..], 5).unwrap(), b"12345");
        assert!(read_capped(&b"123456"[..], 5).unwrap_err().contains("too big"));
    }
}
