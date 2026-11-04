//! The bibliography (R13): sources stored as fields and shown in APA style.
//!
//! Which fields are required is open question Q12, so nothing is enforced yet:
//! missing parts are simply left out, and a missing year shows as "n.d." as APA does.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SourceFields {
    /// As written in APA, e.g. "Kirk, D. B., & Hwu, W. W."
    pub authors: String,
    pub year: Option<String>,
    pub title: String,
    /// e.g. "4th" or "RISC-V", shown as "(4th ed.)"
    pub edition: Option<String>,
    /// e.g. "Lecture slides", shown as "[Lecture slides]"
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub url: Option<String>,
}

fn filled(field: &Option<String>) -> Option<&str> {
    field
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// APA reference as Markdown (the title is in *italics*).
pub fn apa(source: &SourceFields) -> String {
    let authors = source.authors.trim();
    let mut reference = format!(
        "{authors}{} ({}).",
        if authors.ends_with('.') { "" } else { "." },
        filled(&source.year).unwrap_or("n.d.")
    );
    reference.push_str(&format!(" *{}*", source.title.trim()));
    if let Some(edition) = filled(&source.edition) {
        reference.push_str(&format!(" ({edition} ed.)"));
    }
    if let Some(description) = filled(&source.description) {
        reference.push_str(&format!(" [{description}]"));
    }
    reference.push('.');
    if let Some(publisher) = filled(&source.publisher) {
        reference.push_str(&format!(" {publisher}."));
    }
    if let Some(url) = filled(&source.url) {
        reference.push_str(&format!(" {url}"));
    }
    reference
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(text: &str) -> Option<String> {
        Some(text.into())
    }

    // sources.feature: Add a source to the bibliography
    #[test]
    fn formats_a_book_with_an_edition() {
        let source = SourceFields {
            authors: "Patterson, D. A., & Hennessy, J. L.".into(),
            year: some("2020"),
            title: "Computer organization and design".into(),
            edition: some("RISC-V"),
            publisher: some("Morgan Kaufmann"),
            ..Default::default()
        };
        assert_eq!(
            apa(&source),
            "Patterson, D. A., & Hennessy, J. L. (2020). *Computer organization and design* (RISC-V ed.). Morgan Kaufmann."
        );
    }

    #[test]
    fn formats_lecture_slides_with_a_description() {
        let source = SourceFields {
            authors: "University College Isle of Man".into(),
            year: some("2026"),
            title: "Lecture 3: The processor pipeline".into(),
            description: some("Lecture slides"),
            ..Default::default()
        };
        assert_eq!(
            apa(&source),
            "University College Isle of Man. (2026). *Lecture 3: The processor pipeline* [Lecture slides]."
        );
    }

    #[test]
    fn missing_year_is_no_date_and_url_goes_last() {
        let source = SourceFields {
            authors: "Mozilla".into(),
            title: "PDF.js".into(),
            url: some("https://mozilla.github.io/pdf.js/"),
            ..Default::default()
        };
        assert_eq!(
            apa(&source),
            "Mozilla. (n.d.). *PDF.js*. https://mozilla.github.io/pdf.js/"
        );
    }

    #[test]
    fn blank_optional_fields_are_left_out() {
        let source = SourceFields {
            authors: "Kirk, D. B., & Hwu, W. W.".into(),
            year: some(" 2022 "),
            title: "Programming massively parallel processors".into(),
            edition: some("  "),
            publisher: some(""),
            ..Default::default()
        };
        assert_eq!(
            apa(&source),
            "Kirk, D. B., & Hwu, W. W. (2022). *Programming massively parallel processors*."
        );
    }
}
