//! `cargo xtask web-check [dist-dir]`: the built web site loads nothing from another origin and carries no ArtCraft image.
//!
//! The site is a static page on GitHub Pages and has to work offline once loaded, with no CDN, font
//! host or tracker. `index.html` may not name any absolute URL, and the generated JavaScript may
//! only mention the XML namespace strings it needs.

use std::path::Path;

use anyhow::{Context, bail};

use crate::gates;

/// URL prefixes that appear in generated glue as namespace identifiers (`createElementNS`) and are
/// never fetched.
const NAMESPACE_PREFIXES: &[&str] = &["http://www.w3.org/", "https://www.w3.org/"];

/// How many bytes of each brand image [`check_wasm`] looks for.
const PROBE: usize = 48;

/// The image addresses the desktop build registers for the ArtCraft marks; the web build has none.
const BRAND_URIS: &[&[u8]] = &[b"bytes://artcraft"];

/// What `index.html` may not contain at all.
const FORBIDDEN_IN_INDEX: &[&str] = &["http://", "https://", "=\"//", "='//", "url(//", "url(\"//", "url('//", "@import"];

pub fn run(args: &[String]) -> anyhow::Result<()> {
    let dir = gates::root().join(args.first().map_or("dist/web", String::as_str));
    let index = std::fs::read_to_string(dir.join("index.html")).with_context(|| format!("reading {}", dir.join("index.html").display()))?;
    let mut problems: Vec<String> = Vec::new();
    if let Err(found) = check_index(&index) {
        problems.extend(found.into_iter().map(|p| format!("index.html: {p}")));
    }
    let images = brand_images()?;
    let (mut scripts, mut modules) = (0usize, 0usize);
    for entry in std::fs::read_dir(&dir).with_context(|| format!("listing {}", dir.display()))? {
        let path = entry?.path();
        match path.extension().and_then(|e| e.to_str()) {
            Some("js") => {
                scripts += 1;
                problems.extend(check_script(&path)?);
            }
            Some("wasm") => {
                modules += 1;
                problems.extend(check_module(&path, &images)?);
            }
            _ => {}
        }
    }
    if !problems.is_empty() {
        bail!("the built site is not self-contained or carries brand images:\n  {}", problems.join("\n  "));
    }
    println!(
        "web-check: index.html, {scripts} script(s) and {modules} wasm module(s) in {} name no outside address and carry no ArtCraft image",
        dir.display()
    );
    Ok(())
}

/// The ArtCraft brand SVGs in `docs/brand/`, which the web build must not carry.
fn brand_images() -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let dir = gates::root().join("docs/brand");
    let mut images = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("listing {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "svg") {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            images.push((name, std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?));
        }
    }
    Ok(images)
}

fn check_module(path: &Path, images: &[(String, Vec<u8>)]) -> anyhow::Result<Vec<String>> {
    let wasm = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(check_wasm(&wasm, images).err().unwrap_or_default().into_iter().map(|p| format!("{name}: {p}")).collect())
}

fn check_script(path: &Path) -> anyhow::Result<Vec<String>> {
    let js = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(check_glue(&js).err().unwrap_or_default().into_iter().map(|p| format!("{name}: {p}")).collect())
}

/// `index.html` is ours (plus the loader Trunk injects, which uses relative paths): any absolute or
/// protocol-relative address in it is a mistake.
pub(crate) fn check_index(html: &str) -> Result<(), Vec<String>> {
    let found: Vec<String> =
        FORBIDDEN_IN_INDEX.iter().filter(|needle| html.contains(**needle)).map(|needle| format!("contains `{needle}`")).collect();
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// Generated JavaScript: every `http(s)://` address must start with one of [`NAMESPACE_PREFIXES`].
pub(crate) fn check_glue(js: &str) -> Result<(), Vec<String>> {
    let mut found = Vec::new();
    for scheme in ["http://", "https://"] {
        for (at, _) in js.match_indices(scheme) {
            let rest = js.get(at..).unwrap_or_default();
            let end = rest.find(['"', '\'', '`', ' ', ')', '\n']).unwrap_or(rest.len());
            let url = rest.get(..end).unwrap_or(rest);
            if !NAMESPACE_PREFIXES.iter().any(|p| url.starts_with(p)) {
                found.push(format!("mentions `{}`", url.chars().take(80).collect::<String>()));
            }
        }
    }
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// The built wasm must not carry ArtCraft's brand images (docs/brand/LICENSE-brand.txt): `images` are
/// the brand SVG files, each looked for by a slice taken from its middle.
pub(crate) fn check_wasm(wasm: &[u8], images: &[(String, Vec<u8>)]) -> Result<(), Vec<String>> {
    let mut found = Vec::new();
    for (name, svg) in images {
        // A slice from the middle: the first bytes of every SVG are the same XML boilerplate.
        let len = svg.len().min(PROBE);
        let start = svg.len().saturating_sub(len) / 2;
        let probe = svg.get(start..start.saturating_add(len)).unwrap_or_default();
        if contains(wasm, probe) {
            found.push(format!("carries the brand image {name}"));
        }
    }
    for uri in BRAND_URIS {
        if contains(wasm, uri) {
            found.push(format!("names `{}`", String::from_utf8_lossy(uri)));
        }
    }
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// Whether `needle` (not empty) occurs in `haystack`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brand() -> Vec<(String, Vec<u8>)> {
        vec![("artcraft-mark.svg".to_owned(), b"<svg><title>ArtCraft</title><path d=\"M104.28,49.49L81.55,0h-31.23l-3.17,4.76\"/></svg>".to_vec())]
    }

    #[test]
    fn rejects_a_wasm_that_carries_a_brand_image() {
        let mut wasm = vec![0u8; 64];
        wasm.extend_from_slice(&brand()[0].1);
        wasm.extend_from_slice(&[7u8; 64]);
        let problems = check_wasm(&wasm, &brand()).unwrap_err();
        assert!(problems[0].contains("artcraft-mark.svg"), "{problems:?}");
    }

    #[test]
    fn rejects_a_wasm_that_names_the_brand_image_uri() {
        assert!(check_wasm(b"..bytes://artcraft-logo.svg..", &brand()).is_err());
    }

    #[test]
    fn accepts_a_wasm_without_brand_images() {
        assert!(check_wasm(&[1u8; 4096], &brand()).is_ok());
        assert!(check_wasm(&[], &brand()).is_ok());
        assert!(check_wasm(&[1u8; 16], &[]).is_ok(), "no images to look for");
        assert!(check_wasm(&[1u8; 16], &[("empty.svg".to_owned(), Vec::new())]).is_ok(), "an empty image matches nothing");
    }

    #[test]
    fn rejects_an_absolute_script_src() {
        let html = r#"<script src="https://cdn.example.com/x.js"></script>"#;
        assert!(check_index(html).is_err());
    }

    #[test]
    fn rejects_a_protocol_relative_src() {
        assert!(check_index(r#"<script src="//cdn.example.com/x.js"></script>"#).is_err());
    }

    #[test]
    fn rejects_a_google_fonts_link() {
        let html = r#"<link href="https://fonts.googleapis.com/css2?family=Inter" rel="stylesheet">"#;
        let problems = check_index(html).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("https://")), "{problems:?}");
        assert!(check_index("<style>@import url(x.css);</style>").is_err());
    }

    #[test]
    fn accepts_relative_assets() {
        let html =
            r#"<link rel="preload" href="./compilo-abc_bg.wasm" as="fetch"><script type="module">import init from './compilo-abc.js';</script>"#;
        assert!(check_index(html).is_ok());
    }

    #[test]
    fn glue_may_name_the_svg_namespace_but_not_fetch_elsewhere() {
        assert!(check_glue(r#"document.createElementNS("http://www.w3.org/2000/svg", "svg")"#).is_ok());
        let problems = check_glue(r#"fetch("https://example.com/a"); x = "http://www.w3.org/1999/xhtml""#).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("example.com"));
    }

    #[test]
    fn glue_check_handles_text_that_ends_in_a_scheme() {
        assert!(check_glue("see http://").is_err());
        assert!(check_glue("").is_ok());
    }
}
