//! Turning other apps' text into the app's Markdown and back (import-export.feature, R171
//! to R176). Anki stores HTML; the app stores Markdown with `$…$` maths and `image:` links.

/// What `html_to_markdown` found that it couldn't keep.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Dropped {
    pub sounds: usize,
}

fn entity(name: &str) -> Option<String> {
    let named = match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" | "#39" => "'",
        "nbsp" => " ",
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            return char::from_u32(code).map(String::from);
        }
    };
    Some(named.to_string())
}

/// Decodes `&amp;`, `&#39;` and the like.
pub fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match rest[1..].find(';').filter(|end| *end <= 10).and_then(|end| entity(&rest[1..=end]).map(|text| (text, end))) {
            Some((text, end)) => {
                out.push_str(&text);
                rest = &rest[end + 2..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let at = lower.find(&format!("{name}="))? + name.len() + 1;
    let value = &tag[at..];
    let (quote, value) = match value.chars().next()? {
        quote @ ('"' | '\'') => (Some(quote), &value[1..]),
        _ => (None, value),
    };
    let end = match quote {
        Some(quote) => value.find(quote)?,
        None => value.find(|letter: char| letter.is_whitespace() || letter == '>').unwrap_or(value.len()),
    };
    Some(decode_entities(&value[..end]))
}

/// Anki's HTML as the app's Markdown. `image` turns a media file name (and its description,
/// if it has one) into the Markdown for it, or nothing when it isn't an image the app can show. Sounds are dropped and counted.
pub fn html_to_markdown(html: &str, image: &mut dyn FnMut(&str, Option<&str>) -> Option<String>, dropped: &mut Dropped) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while !rest.is_empty() {
        if rest.starts_with("[sound:") {
            if let Some(end) = rest.find(']') {
                dropped.sounds += 1;
                rest = &rest[end + 1..];
                continue;
            }
        }
        if rest.starts_with('<') {
            let Some(end) = rest.find('>') else {
                out.push_str(rest);
                break;
            };
            let tag = &rest[1..end];
            rest = &rest[end + 1..];
            let closing = tag.starts_with('/');
            let name = tag.trim_start_matches('/').split(|letter: char| letter.is_whitespace() || letter == '/').next().unwrap_or("").to_ascii_lowercase();
            match name.as_str() {
                "br" => out.push('\n'),
                "div" | "p" | "tr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "table" | "blockquote" | "hr" => {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
                "li" if !closing => {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("- ");
                }
                "td" | "th" if closing => out.push(' '),
                "b" | "strong" => out.push_str("**"),
                "i" | "em" => out.push('*'),
                "code" => out.push('`'),
                "img" if !closing => {
                    let alt = attribute(tag, "alt");
                    if let Some(markdown) = attribute(tag, "src").and_then(|source| image(&source, alt.as_deref())) {
                        out.push_str(&markdown);
                    }
                }
                "script" | "style" if !closing => {
                    // Their contents are never text.
                    let close = format!("</{name}");
                    rest = rest.to_ascii_lowercase().find(&close).map(|at| &rest[at..]).unwrap_or("");
                }
                _ => {}
            }
            continue;
        }
        let next = rest.find(['<', '[']).map(|at| if at == 0 { 1 } else { at }).unwrap_or(rest.len());
        out.push_str(&decode_entities(&rest[..next]));
        rest = &rest[next..];
    }
    // MathJax delimiters as the app's.
    let out = out.replace("\\(", "$").replace("\\)", "$").replace("\\[", "$$").replace("\\]", "$$");
    let mut tidy = String::with_capacity(out.len());
    let mut blank = 0;
    for line in out.lines() {
        // Things taken out (sounds, unusable images) leave runs of spaces: one is enough.
        let indent = line.len() - line.trim_start().len();
        let mut squeezed = line[..indent].to_string();
        for (index, word) in line[indent..].split(' ').filter(|word| !word.is_empty()).enumerate() {
            if index > 0 {
                squeezed.push(' ');
            }
            squeezed.push_str(word);
        }
        let line = squeezed.as_str();
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        tidy.push_str(line);
        tidy.push('\n');
    }
    tidy.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(html: &str) -> (String, Dropped) {
        let mut dropped = Dropped::default();
        let text = html_to_markdown(html, &mut |name, alt| (name.ends_with(".png")).then(|| format!("![{}](image:u1)", alt.unwrap_or(name.trim_end_matches(".png")))), &mut dropped);
        (text, dropped)
    }

    #[test]
    fn formatting_line_breaks_and_entities_become_markdown() {
        assert_eq!(convert("What is <b>DNS</b>?<br>A <i>name</i> &amp; <em>number</em> lookup&nbsp;system").0, "What is **DNS**?\nA *name* & *number* lookup system");
        assert_eq!(convert("<div>One</div><div>Two</div>").0, "One\nTwo");
        assert_eq!(convert("<ul><li>A</li><li>B</li></ul>").0, "- A\n- B");
        assert_eq!(convert("x &lt; y &#8212; &#x2014;").0, "x < y — —");
    }

    #[test]
    fn images_become_image_links_and_sounds_are_dropped() {
        let (text, dropped) = convert(r#"The TLB <img src="tlb.png"> and <img src='gone.svg'> [sound:click.mp3] <img alt="the diagram" src="d.png">"#);
        assert_eq!(text, "The TLB ![tlb](image:u1) and ![the diagram](image:u1)");
        assert_eq!(dropped.sounds, 1);
    }

    #[test]
    fn mathjax_cloze_and_scripts() {
        assert_eq!(convert(r"Area \(\pi r^2\) and \[E=mc^2\]").0, "Area $\\pi r^2$ and $$E=mc^2$$");
        assert_eq!(convert("A {{c1::cipher::hint}}").0, "A {{c1::cipher::hint}}");
        assert_eq!(convert("<style>.x{}</style>Text<script>alert(1)</script>").0, "Text");
    }
}
