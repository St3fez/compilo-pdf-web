//! `cargo xtask web-check [dist-dir]`: the built web site loads nothing from another origin, carries no
//! ArtCraft image and ships its licence documents, its OCR models and its Search Console file.
//! `cargo xtask web-licences` puts the documents beside it, `cargo xtask models dist/web/models` the
//! models.
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

/// The notices that sit at the top of the site, next to `index.html`.
const TOP_NOTICES: &[&str] = &["LICENSE-MIT", "LICENSE-APACHE", "NOTICE"];

/// How many bytes of each brand image [`check_wasm`] looks for.
const PROBE: usize = 48;

/// The image addresses the desktop build registers for the ArtCraft marks; the web build has none.
const BRAND_URIS: &[&[u8]] = &[b"bytes://artcraft"];

/// What `index.html` may not contain at all.
const FORBIDDEN_IN_INDEX: &[&str] = &["=\"//", "='//", "url(//", "url(\"//", "url('//", "@import"];

/// The absolute addresses `index.html` may name, which search engines read and the page never
/// fetches: the site's own (its canonical address and preview image) and schema.org, the vocabulary
/// of its structured data. Each is a whole address or ends where a path begins.
const INDEX_OWN_ADDRESSES: &[&str] = &["https://st3fez.github.io/compilo-pdf-web/", "https://schema.org"];

pub fn run(args: &[String]) -> anyhow::Result<()> {
    let dir = gates::root().join(args.first().map_or("dist/web", String::as_str));
    let index = std::fs::read_to_string(dir.join("index.html")).with_context(|| format!("reading {}", dir.join("index.html").display()))?;
    let mut problems: Vec<String> = Vec::new();
    for found in [check_index(&index), check_own_files(&dir, &index)].into_iter().filter_map(Result::err) {
        problems.extend(found.into_iter().map(|p| format!("index.html: {p}")));
    }
    if let Err(found) = check_tree(&gates::root()) {
        problems.extend(found);
    }
    if let Err(found) = check_notices(&dir, &license_documents(&gates::root())?) {
        problems.extend(found);
    }
    let manifest = crate::assets::load(&gates::root())?;
    let models: Vec<(String, String)> = manifest.fetched.iter().filter(|f| f.kind == "model").map(|f| (f.file.clone(), f.sha256.clone())).collect();
    if let Err(found) = check_models(&dir, &models) {
        problems.extend(found);
    }
    if let Err(found) = check_search_console(&dir) {
        problems.extend(found);
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
        bail!(
            "the built site is not self-contained, carries brand images or lacks its licence documents, OCR models or Search Console file:\n  {}",
            problems.join("\n  ")
        );
    }
    println!(
        "web-check: index.html, {scripts} script(s) and {modules} wasm module(s) in {} name no outside address and carry no ArtCraft image; the licence documents and the {} OCR model(s) with their licences are beside them",
        dir.display(),
        models.len()
    );
    Ok(())
}

/// The ArtCraft brand SVGs in `docs/brand/`, which the web build must not carry. This fork has none
/// (see [`check_tree`]); a checkout that still has them is probed for them too.
fn brand_images() -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    brand_images_in(&gates::root().join("docs/brand"))
}

fn brand_images_in(dir: &Path) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let mut images = Vec::new();
    if !dir.is_dir() {
        return Ok(images);
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("listing {}", dir.display()))? {
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
/// protocol-relative address in it but [`INDEX_OWN_ADDRESSES`] is a mistake.
pub(crate) fn check_index(html: &str) -> Result<(), Vec<String>> {
    let mut found: Vec<String> =
        FORBIDDEN_IN_INDEX.iter().filter(|needle| html.contains(**needle)).map(|needle| format!("contains `{needle}`")).collect();
    for scheme in ["http://", "https://"] {
        for (at, _) in html.match_indices(scheme) {
            let url = address_at(html.get(at..).unwrap_or_default());
            let own =
                |allowed: &&str| url.strip_prefix(*allowed).is_some_and(|rest| rest.is_empty() || allowed.ends_with('/') || rest.starts_with('/'));
            if !INDEX_OWN_ADDRESSES.iter().any(own) {
                found.push(format!("names `{}`", url.chars().take(80).collect::<String>()));
            }
        }
    }
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// The files `index.html` names by the site's own address (its preview image): each must be in
/// `site`, or what a search engine or a link preview shows is broken.
pub(crate) fn check_own_files(site: &Path, html: &str) -> Result<(), Vec<String>> {
    let Some(base) = INDEX_OWN_ADDRESSES.first() else { return Ok(()) };
    let missing: Vec<String> = html
        .match_indices(base)
        .filter_map(|(at, _)| {
            let path = address_at(html.get(at + base.len()..).unwrap_or_default());
            (!path.is_empty() && !site.join(path).is_file()).then(|| format!("names {base}{path}, which the site lacks"))
        })
        .collect();
    if missing.is_empty() { Ok(()) } else { Err(missing) }
}

/// The address at the start of `text`, up to the quote, space or bracket that ends it.
fn address_at(text: &str) -> &str {
    let end = text.find(['"', '\'', '`', ' ', ')', '<', '>', '\n']).unwrap_or(text.len());
    text.get(..end).unwrap_or(text)
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

/// The ArtCraft marks must not be in the checkout at all (docs/brand/LICENSE-brand.txt: a published
/// modified version removes them).
pub(crate) fn check_tree(root: &Path) -> Result<(), Vec<String>> {
    let Ok(entries) = std::fs::read_dir(root.join("docs/brand")) else { return Ok(()) };
    let mut found: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.to_ascii_lowercase().starts_with("artcraft"))
        .map(|name| format!("docs/brand/{name} is an ArtCraft mark, which this fork does not carry"))
        .collect();
    found.sort();
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// The licence documents the site must carry beside the wasm: `site` holds the three top-level notices
/// and every `LICENSES.toml` document under `licenses/`.
pub(crate) fn check_notices(site: &Path, documents: &[(String, String)]) -> Result<(), Vec<String>> {
    let wanted = TOP_NOTICES.iter().map(|name| (*name).to_owned()).chain(documents.iter().map(|(_, destination)| format!("licenses/{destination}")));
    let missing: Vec<String> = wanted
        .filter(|relative| !std::fs::metadata(site.join(relative)).is_ok_and(|m| m.is_file() && m.len() > 0))
        .map(|relative| format!("the licence document {relative} is missing or empty (run `cargo xtask web-licences`)"))
        .collect();
    if missing.is_empty() { Ok(()) } else { Err(missing) }
}

/// The text recognition models in `site/models/`, where Recognize text downloads them from: each of
/// `models` (`(file, sha256)` from ATTRIBUTION.toml) as pinned, with its licence beside it, and the
/// credits their licence asks for (`cargo xtask models` writes all three).
pub(crate) fn check_models(site: &Path, models: &[(String, String)]) -> Result<(), Vec<String>> {
    let dir = site.join("models");
    let present = |name: &str| std::fs::metadata(dir.join(name)).is_ok_and(|m| m.is_file() && m.len() > 0);
    let mut problems = Vec::new();
    for (file, sha256) in models {
        match std::fs::read(dir.join(file)) {
            Ok(bytes) if crate::assets::sha256_hex(&bytes) == *sha256 => {}
            Ok(_) => problems.push(format!("models/{file}: SHA-256 does not match ATTRIBUTION.toml")),
            Err(_) => problems.push(format!("the model models/{file} is missing (run `cargo xtask models dist/web/models`)")),
        }
        let licence = format!("{file}.LICENCE.txt");
        if !present(&licence) {
            problems.push(format!("the licence models/{licence} is missing or empty"));
        }
    }
    if !present("ATTRIBUTION.txt") {
        problems.push("the credits models/ATTRIBUTION.txt are missing or empty".to_owned());
    }
    if problems.is_empty() { Ok(()) } else { Err(problems) }
}

/// The file Google Search Console fetches to confirm the site is the owner's
/// (https://st3fez.github.io/compilo-pdf-web/ is a URL-prefix property verified by HTML file).
/// `index.html` copies it into the site; deleting it later unverifies the property.
const SEARCH_CONSOLE_FILE: &str = "google07d7cf89ae939f6f.html";

/// `site` serves [`SEARCH_CONSOLE_FILE`] with the one line Google wrote in it.
pub(crate) fn check_search_console(site: &Path) -> Result<(), Vec<String>> {
    match std::fs::read_to_string(site.join(SEARCH_CONSOLE_FILE)) {
        Ok(text) if text.trim_end() == format!("google-site-verification: {SEARCH_CONSOLE_FILE}") => Ok(()),
        Ok(_) => Err(vec![format!("{SEARCH_CONSOLE_FILE} does not hold the line Google Search Console wrote in it")]),
        Err(_) => Err(vec![format!("{SEARCH_CONSOLE_FILE}, which Google Search Console fetches to verify the site, is missing")]),
    }
}

/// Copies the licence documents (`(source, destination)` pairs of `LICENSES.toml`) from the checkout
/// `root` into `site`, as the release zip does with `copy_docs`; returns how many files it wrote.
pub(crate) fn copy_notices(root: &Path, site: &Path, documents: &[(String, String)]) -> anyhow::Result<usize> {
    let pairs = TOP_NOTICES
        .iter()
        .map(|name| ((*name).to_owned(), (*name).to_owned()))
        .chain(documents.iter().map(|(source, destination)| (source.clone(), format!("licenses/{destination}"))));
    let mut copied = 0;
    for (source, target) in pairs {
        if target.split(['/', '\\']).any(|part| part == ".." || part.is_empty()) || Path::new(&target).is_absolute() {
            bail!("unsafe licence document destination {target}");
        }
        let to = site.join(&target);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::copy(root.join(&source), &to).with_context(|| format!("copying {source} to {}", to.display()))?;
        copied += 1;
    }
    Ok(copied)
}

/// The `(source, destination)` of every `[[document]]` in `LICENSES.toml`.
fn license_documents(root: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let path = root.join("LICENSES.toml");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let table: toml::Table = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let documents = table.get("document").and_then(toml::Value::as_array).context("LICENSES.toml has no [[document]] entries")?;
    documents
        .iter()
        .map(|document| {
            let field = |name: &str| {
                document
                    .get(name)
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
                    .with_context(|| format!("a LICENSES.toml document has no `{name}`"))
            };
            Ok((field("source")?, field("destination")?))
        })
        .collect()
}

/// `cargo xtask web-licences [dist-dir]`: puts the licence documents beside the built site, so a copy
/// of the wasm (which embeds fonts, icons and other people's code) always travels with its notices.
pub fn licences(args: &[String]) -> anyhow::Result<()> {
    let root = gates::root();
    let site = root.join(args.first().map_or("dist/web", String::as_str));
    if !site.join("index.html").is_file() {
        bail!("{} has no index.html: build the site first (trunk build --release in apps/pdfcraft-web)", site.display());
    }
    let copied = copy_notices(&root, &site, &license_documents(&root)?)?;
    println!("web-licences: {copied} licence document(s) copied into {}", site.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn scratch() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("pdfcraft-web-check-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A checkout with the notices and one font licence, and the `LICENSES.toml` pairs naming them.
    fn checkout() -> (PathBuf, Vec<(String, String)>) {
        let root = scratch();
        let documents = ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "assets/fonts/OFL-Inter.txt"];
        for d in documents {
            write(&root, d, &format!("text of {d}\n"));
        }
        (root, documents.iter().map(|d| ((*d).to_owned(), (*d).to_owned())).collect())
    }

    #[test]
    fn a_checkout_with_a_brand_mark_is_rejected() {
        for file in ["docs/brand/artcraft-mark.svg", "docs/brand/artcraft-logo-white.png"] {
            let root = scratch();
            write(&root, file, "<svg/>");
            let problems = check_tree(&root).unwrap_err();
            assert!(problems.iter().any(|p| p.contains(file)), "{file}: {problems:?}");
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn a_checkout_without_brand_marks_is_accepted() {
        let root = scratch();
        assert!(check_tree(&root).is_ok(), "no docs/brand at all");
        write(&root, "docs/brand/README.txt", "our own artwork goes here");
        assert!(check_tree(&root).is_ok(), "other files in docs/brand are not ArtCraft marks");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn there_are_no_brand_images_to_look_for_when_docs_brand_is_gone() {
        let root = scratch();
        assert!(brand_images_in(&root.join("docs/brand")).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Recognize text downloads its models from `models/` beside the page: they must be there, as
    /// ATTRIBUTION.toml pins them, with their CC-BY-SA licence and the credits it asks for.
    #[test]
    fn a_site_without_its_ocr_models_their_licences_or_credits_is_rejected() {
        let site = scratch();
        let models = [("text-detection.onnx".to_owned(), crate::assets::sha256_hex(b"model"))];
        let problems = check_models(&site, &models).unwrap_err();
        for missing in ["models/text-detection.onnx", "models/text-detection.onnx.LICENCE.txt", "models/ATTRIBUTION.txt"] {
            assert!(problems.iter().any(|p| p.contains(missing)), "{missing} not reported: {problems:?}");
        }
        write(&site, "models/text-detection.onnx", "another model");
        write(&site, "models/text-detection.onnx.LICENCE.txt", "Attribution-ShareAlike 4.0 International");
        write(&site, "models/ATTRIBUTION.txt", "credits");
        let problems = check_models(&site, &models).unwrap_err();
        assert!(problems.len() == 1 && problems[0].contains("SHA-256"), "{problems:?}");
        write(&site, "models/text-detection.onnx", "model");
        assert!(check_models(&site, &models).is_ok());
        std::fs::remove_dir_all(site).unwrap();
    }

    #[test]
    fn a_site_without_the_licence_documents_is_rejected() {
        let (root, documents) = checkout();
        let site = scratch();
        write(&site, "NOTICE", "only this one\n");
        let problems = check_notices(&site, &documents).unwrap_err();
        for missing in ["LICENSE-MIT", "LICENSE-APACHE", "licenses/assets/fonts/OFL-Inter.txt"] {
            assert!(problems.iter().any(|p| p.contains(missing)), "{missing} not reported: {problems:?}");
        }
        assert!(!problems.iter().any(|p| p.contains("NOTICE") && !p.contains("licenses/NOTICE")), "the top-level NOTICE is there: {problems:?}");
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(site).unwrap();
    }

    #[test]
    fn an_empty_licence_document_does_not_count() {
        let (root, documents) = checkout();
        let site = scratch();
        copy_notices(&root, &site, &documents).unwrap();
        write(&site, "licenses/assets/fonts/OFL-Inter.txt", "");
        let problems = check_notices(&site, &documents).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("OFL-Inter.txt")), "{problems:?}");
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(site).unwrap();
    }

    #[test]
    fn copied_notices_satisfy_the_check() {
        let (root, documents) = checkout();
        let site = scratch();
        let copied = copy_notices(&root, &site, &documents).unwrap();
        assert_eq!(copied, 3 + documents.len(), "the three top-level notices and every document");
        assert_eq!(std::fs::read_to_string(site.join("licenses/assets/fonts/OFL-Inter.txt")).unwrap(), "text of assets/fonts/OFL-Inter.txt\n");
        assert_eq!(std::fs::read_to_string(site.join("NOTICE")).unwrap(), "text of NOTICE\n");
        assert!(check_notices(&site, &documents).is_ok());
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(site).unwrap();
    }

    #[test]
    fn copying_a_document_that_is_missing_names_it() {
        let (root, mut documents) = checkout();
        documents.push(("assets/fonts/OFL-Gone.txt".to_owned(), "assets/fonts/OFL-Gone.txt".to_owned()));
        let site = scratch();
        let error = copy_notices(&root, &site, &documents).unwrap_err();
        assert!(format!("{error:#}").contains("OFL-Gone.txt"), "{error:#}");
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(site).unwrap();
    }

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

    /// Search engines read the page's canonical address, its preview image and its structured data:
    /// `index.html` may name the site's own address and schema.org's vocabulary, which the page never
    /// fetches, and no other absolute address.
    #[test]
    fn index_may_name_only_the_site_s_own_address_and_schema_org() {
        let html = r#"<link rel="canonical" href="https://st3fez.github.io/compilo-pdf-web/"><meta property="og:image" content="https://st3fez.github.io/compilo-pdf-web/og-image.png"><script type="application/ld+json">{"@context":"https://schema.org","@type":"WebApplication"}</script>"#;
        assert!(check_index(html).is_ok(), "{:?}", check_index(html));
        let problems = check_index(r#"<meta property="og:image" content="https://cdn.example.com/x.png">"#).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("cdn.example.com")), "{problems:?}");
        for look_alike in
            ["https://st3fez.github.io/compilo-pdf-web.evil.example/", "https://schema.org.evil.example/", "http://st3fez.github.io/compilo-pdf-web/"]
        {
            assert!(check_index(&format!(r#"<a href="{look_alike}">"#)).is_err(), "{look_alike} is not the site");
        }
    }

    /// Google Search Console confirms the site is the owner's by fetching this file: the site must
    /// carry it, as Google wrote it, or the property loses its verification.
    #[test]
    fn a_site_without_its_search_console_file_is_rejected() {
        let site = scratch();
        assert!(check_search_console(&site).is_err(), "no file");
        write(&site, SEARCH_CONSOLE_FILE, "google-site-verification: google0123456789abcdef.html");
        assert!(check_search_console(&site).is_err(), "another property's text");
        write(&site, SEARCH_CONSOLE_FILE, &format!("google-site-verification: {SEARCH_CONSOLE_FILE}"));
        assert!(check_search_console(&site).is_ok(), "{:?}", check_search_console(&site));
        std::fs::remove_dir_all(site).unwrap();
    }

    #[test]
    fn the_web_app_copies_its_search_console_file_into_the_site() {
        let app = gates::root().join("apps/pdfcraft-web");
        assert!(check_search_console(&app).is_ok(), "{:?}", check_search_console(&app));
        let index = std::fs::read_to_string(app.join("index.html")).unwrap();
        assert!(index.contains(&format!(r#"<link data-trunk rel="copy-file" href="{SEARCH_CONSOLE_FILE}" />"#)), "index.html does not copy it");
    }

    /// A file `index.html` names by the site's own address (the preview image) must be on the site.
    #[test]
    fn a_file_named_by_the_site_s_address_must_be_on_the_site() {
        let site = scratch();
        write(&site, "og-image.png", "png");
        let html = r#"<link rel="canonical" href="https://st3fez.github.io/compilo-pdf-web/"><meta property="og:image" content="https://st3fez.github.io/compilo-pdf-web/og-image.png"><meta content="https://st3fez.github.io/compilo-pdf-web/gone.png">"#;
        let problems = check_own_files(&site, html).unwrap_err();
        assert!(problems.len() == 1 && problems[0].contains("gone.png"), "{problems:?}");
        std::fs::remove_dir_all(site).unwrap();
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
