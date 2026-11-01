# Compilo PDF Neon Web Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish the real PdfCraft editor (egui compiled to WASM), renamed **Compilo PDF**, as a neon-themed public site on `https://st3fez.github.io/compilo-pdf-web/`, built and deployed by GitHub Actions in the owner's fork `St3fez/compilo-pdf-web`.

**Architecture:** A new `ThemeKind::Neon` palette in `ui-egui` (new files, minimal edits to existing ones), switched on for the web build by a `web_profile` flag on `PdfCraftApp` that also removes the ArtCraft marks, swaps in the "B" Home (drop zone + eight tool tiles) and marks the desktop-only features. `apps/pdfcraft-web` calls the flag; `index.html` carries the matching CSS; one fork-only workflow builds with Trunk and deploys to Pages.

**Tech Stack:** Rust 1.95 (edition 2024), egui/eframe (glow), Trunk, wasm-bindgen, GitHub Actions + Pages, `egui_kittest`.

**Spec:** `docs/superpowers/specs/2026-10-30-github-pages-neon-web-design.md`. Repo rules that override it: `CLAUDE.md`, `AGENTS.md`.

## Global Constraints

- Rust only; no handwritten JS/TS. `index.html` may carry CSS. Generated Trunk glue is fine.
- Never panic on any input: no `unwrap`/`expect`/`panic!` in non-test code, `get()` not `[i]`, checked arithmetic. `unsafe` forbidden.
- Layering: the theme lives in `ui-egui` (L7). Nothing below L7 depends on egui.
- No font files committed. No assets from Adobe. This plan adds **no** image/font/icon file, so no `ATTRIBUTION.toml` entry is needed; if a task finds it needs one, stop and add it (`cargo xtask assets --write`, then `cargo xtask assets`). No inline-SVG favicon: it counts as an asset.
- No ArtCraft name, wordmark or mark in the published build (`docs/brand/LICENSE-brand.txt`). The only allowed mention is the plain-text credit "Compilo PDF is based on PdfCraft by the ArtCraft team". The desktop build keeps everything unchanged.
- The product name a visitor sees is **Compilo PDF** (decided by the owner 2026-11-01), never "PdfCraft", except in that credit.
- Palette (spec): cyan `#4dfcff`, magenta `#ff2bd6`, violet `#b46bff`, lime `#b6ff3b` on a near-black violet `#0b0716`. Pages stay white with a soft glow; documents are never recoloured. Body and control text keep WCAG AA contrast (4.5:1).
- Desktop Light and Dark look and behaviour unchanged. Neon is selectable in every build, default only in the web build.
- No PDF leaves the browser; no runtime request to CDNs or font hosts; the site works offline once loaded.
- Site served from `/compilo-pdf-web/`: every URL relative (`Trunk.toml` already has `public_url = "./"`).
- Branch from an up-to-date `main` as `<type>/<slug>`, one task per commit, commit only green states. Gates before each commit: `cargo fmt --check`; `timeout 1800 cargo clippy -p <crate> -j 2 --all-targets -- -D warnings`; `timeout 1800 cargo test -p <crate> -j 2`; `cargo xtask layers`; `cargo xtask assets`; `cargo xtask wasm`. Never run a `--workspace` build locally.
- Workflows pin actions by commit SHA. CLAUDE.md says not to add CI in a feature PR: the Pages workflow goes on its own fork-only branch (Task 7) and is never proposed upstream.
- Never push to or open a PR against `storytold/pdfcraft`. Creating the public fork and enabling Pages (Task 8) need the owner's go-ahead at that step.
- Keep docs true: `ROADMAP.md` progress line, Web row of `docs/hardware-parity.md`.

## Review Focus

Failure modes the spec implies but no feature task exercises by itself, most likely first. Each has a test in the task named.

1. A visitor drops a non-PDF, a corrupt PDF or several files at once on the drop zone: an in-app error or one tab each, never a panic or a blank Home (Task 4).
2. A visitor picks a tool tile, then the open fails: the tool choice must not stick to a later unrelated file (Task 4).
3. A returning visitor has settings saved by an older build (no Neon variant) or a theme name this build doesn't know: they load, and their saved Light/Dark choice is kept (Task 1).
4. A narrow phone viewport (360×640): tiles wrap, the drop zone and every tile stay on screen (Task 4).
5. WebGL missing or blocked: the page shows a readable message instead of a blank canvas (Task 6).

---

### Task 0: Toolchain, branch and baseline build

Nothing is installed on this machine yet (no `cargo`, `rustup`, `trunk`, no MSVC Build Tools). Installing them is a machine-level change: confirm with the owner before running the installs.

**Files:** none in the repo.

- [ ] **Step 1: Install the toolchain.** `winget install Rustlang.Rustup`; `rustup default stable` (must be ≥ 1.95, `rust-version` in `Cargo.toml`); `rustup target add wasm32-unknown-unknown`; `winget install Microsoft.VisualStudio.2022.BuildTools` with the "Desktop development with C++" workload (the native `cargo test`/`xtask` need the MSVC linker); `cargo install --locked trunk`. Open a new shell so PATH updates.
  Check: `cargo --version`, `trunk --version`, `rustup target list --installed` shows `wasm32-unknown-unknown`.
- [ ] **Step 2: Branch.** In `C:\Users\Pc\Desktop\pdfcraft` (already fast-forwarded to `origin/main`): `git switch -c feat/neon-web`. The untracked spec and this plan go in with Task 1's commit.
- [ ] **Step 3: Baseline wasm build.** `cd apps/pdfcraft-web && timeout 3000 trunk build --release`. Expected: `dist/web/index.html` plus a hashed `.wasm` and `.js`. Record the raw `.wasm` size and the gzip size (`gzip -c file.wasm | wc -c`) in `docs/superpowers/plans/baseline.txt` (untracked scratch, not committed).
- [ ] **Step 4: Baseline serve.** `trunk serve` and open `http://127.0.0.1:8766/` in Chrome. Expected: the stock PdfCraft Home renders. Take a screenshot as the "before".
- [ ] **Step 5: Baseline gates are green before any change:** `cargo fmt --check`; `cargo xtask layers`; `cargo xtask assets`; `cargo xtask wasm`. If any is red on a clean `main`, report it to the owner instead of fixing it here.

---

### Task 1: Neon tokens and theme selection

**Files:**
- Create: `crates/ui-egui/src/theme/neon.rs`
- Modify: `crates/ui-egui/src/theme.rs` (enum variants, `Tokens::for_kind`, `Tokens::dark`, `apply`), `crates/ui-egui/src/chrome.rs:294-305`, `crates/ui-egui/src/commands.rs:330-336`, `crates/ui-egui/src/dialogs/appearance.rs`, `crates/ui-egui/src/menus.rs:170,506`, `crates/ui-egui/src/app/control.rs:28-30`, `crates/engine/src/commands.rs:254` (add `view.theme.neon`), `parity/acrobat-features.toml:1671`, the i18n catalogues for the new strings (follow the convention of existing keys; missing keys fall back to English).
- Test: unit tests inside `theme/neon.rs`; `crates/ui-egui/tests/it/theme.rs` (extend; register nothing new).

**Interfaces:**
- Produces: `ThemeKind::Neon`, `ThemePreference::Neon` (both `serde`, existing variants keep their names); `pub struct NeonAccents { pub cyan: Color32, pub magenta: Color32, pub violet: Color32, pub lime: Color32 }` (`Copy`) in `theme/neon.rs`; `Tokens.neon: Option<NeonAccents>` (`None` for Light/Dark, `Some` for Neon); `Tokens::dark()` is `true` for Neon; command id `"view.theme.neon"` selecting `ThemePreference::Neon`.

- [ ] **Step 1: Write failing tests** in `theme/neon.rs`:
  - `neon_text_meets_aa`: for `Tokens::for_kind(ThemeKind::Neon)`, `theme::contrast(t.text, t.panel) >= 4.5`, same for `text` on `card`, `chrome`, `field`, `text_muted` on `panel`, `accent_text` on `panel`, and `readable(...)` pairs the code already uses.
  - `neon_accents_are_the_spec_colors`: `cyan == 0x4dfcff`, `magenta == 0xff2bd6`, `violet == 0xb46bff`, `lime == 0xb6ff3b`, and `pasteboard == 0x0b0716`.
  - `light_and_dark_have_no_accents`: `Tokens::for_kind(Light).neon.is_none()` and Dark.
  - `neon_is_dark_for_egui`: `Tokens::for_kind(Neon).dark()`.
  In `tests/it/theme.rs`: `running_view_theme_neon_selects_neon` (after `app.run_command("view.theme.neon")`: `theme_preference == Neon`, `theme == Neon`), and the two Review Focus 3 tests: `settings_saved_without_neon_still_load` (restore a JSON with `"theme":"Dark"` → `Dark`) and `unknown_saved_theme_is_ignored` (restore `"theme":"Plasma"` → app keeps its default preference, no panic).
- [ ] **Step 2: Run to verify failure.** `timeout 1800 cargo test -p pdfcraft-ui-egui -j 2 neon` → FAIL (variants missing).
- [ ] **Step 3: Implement** `Tokens::for_kind` arm for Neon using the spec colours. Pick `chrome`, `panel`, `card`, `field`, `border`, `text*` values so Step 1 passes; `selected`/`accent_soft` are low-alpha tints of cyan/violet; `accent` = cyan, `radius` 8. In `apply`, Neon uses `Visuals::dark()` as its base and sets `window_stroke`/`selection.stroke` to violet or cyan, `window_shadow` and `popup_shadow` coloured violet (not black). Update every exhaustive `match` listed under Files so it compiles; the cycle in `chrome.rs` becomes System → Light → Dark → Neon → System. The existing `commands.rs:330` toggle (`== ThemeKind::Light`) keeps its meaning.
  In `app/prefs.rs` confirm the `serde_json::from_value::<ThemePreference>` already ignores unknown values (it does: `if let Ok`); leave it.
- [ ] **Step 4: Run to verify pass.** Same command → PASS; then `cargo test -p pdfcraft-ui-egui -j 2 theme` → all existing theme tests still pass (Light and Dark unchanged).
- [ ] **Step 5: Gates, then commit** `feat(ui-egui): add the Neon theme` with the spec and this plan included.

---

### Task 2: Glow primitive and neon chrome

**Files:**
- Modify: `crates/ui-egui/src/theme/neon.rs`, `crates/ui-egui/src/chrome.rs` (top bar underline), `crates/ui-egui/src/canvas.rs` or the module that paints the page shadow (find the `page_shadow` use; the page glow replaces the black shadow when `Tokens.neon.is_some()`), `crates/ui-egui/src/home.rs` (`card_lift` only if it must pick the glow colour).
- Test: unit tests in `theme/neon.rs`; `crates/ui-egui/tests/page_shadows.rs` (extend).

**Interfaces:**
- Produces: `pub fn glow(painter: &egui::Painter, rect: Rect, radius: impl Into<CornerRadius>, color: Color32, strength: f32)` in `theme::neon`. It paints one `egui::Shadow { offset: [0, 0], blur, spread, color }` shape, `blur` and `spread` scaled from `strength` (clamped 0..=1) and capped at `u8::MAX`. It paints nothing when `strength` is not finite or `<= 0.0`.

- [ ] **Step 1: Write failing tests:** `glow_paints_nothing_for_zero_or_nan_strength` (call with `0.0`, `-1.0`, `f32::NAN`, `f32::INFINITY` into a `Painter` on a headless `Context`; assert shape count is 0 for the first three, ≤ 1 and no panic for infinity); `glow_scales_with_strength` (strength 1.0 gives a larger shape rect than 0.4); in `page_shadows.rs`: `neon_page_gets_a_violet_glow_and_stays_white` (page background pixel/fill is `Color32::WHITE`, the shadow colour has non-zero blue and is not black) and `light_page_shadow_is_unchanged` (same assertion as today).
- [ ] **Step 2: Run to verify failure:** `cargo test -p pdfcraft-ui-egui -j 2 glow` and `cargo test -p pdfcraft-ui-egui -j 2 --test page_shadows` → FAIL.
- [ ] **Step 3: Implement** `glow`; use it for the top-bar underline (cyan→magenta, one `glow` per segment, thin), and the page glow (violet). Only when `Tokens::get(ctx).neon` is `Some`. Measure frame cost: run `cargo test -p pdfcraft-ui-egui -j 2 --test perf` and note whether the existing budget still passes; if Neon pushes it over, lower `blur`, don't skip the test.
- [ ] **Step 4: Run to verify pass,** plus a visual check: `cargo run -p pdfcraft-ui-egui --example shot -- neon-editor.png <synthetic.pdf> --scale 1` after setting the theme through the option verb used by `app/control.rs` (`theme=neon`). Look at the PNG: top bar glows, page floats on violet, text legible.
- [ ] **Step 5: Gates, commit** `feat(ui-egui): neon glow on the top bar and page`.

---

### Task 3: Web profile (default Neon, name Compilo PDF, no ArtCraft marks)

**Files:**
- Create: `crates/ui-egui/src/web_profile.rs`
- Modify: `crates/ui-egui/src/lib.rs` (add `mod web_profile;`, the `web_profile: bool` field of `PdfCraftApp`, set `false` in `app.rs:90` area), `crates/ui-egui/src/i18n.rs` (`tr`, `tr_ctx`, `tr_id`: pass their result through `brand`), `crates/ui-egui/src/menus.rs:24` (`APP_NAME`), `crates/ui-egui/src/app.rs:45` (window title), `crates/ui-egui/src/dialogs.rs:1529` (About heading literal), `crates/ui-egui/src/home.rs:35-62` (community card), `crates/ui-egui/src/dialogs.rs:1527,1545` (About: mark and logo), `crates/ui-egui/src/menus.rs:227` and `chrome.rs:314` (Help items `help.discord`, `help.website`, `help.app_page`), `apps/pdfcraft-web/src/main.rs` (call it), `crates/ui-egui/examples/shot.rs` (accept `--web-profile true` through `set_option`).
- Test: `crates/ui-egui/tests/it/web_profile.rs` (register it in the `it` test root the way `theme.rs` is); unit tests in `i18n/tests.rs`.

**Interfaces:**
- Produces: `PdfCraftApp::set_web_profile(&mut self)` — sets `web_profile = true`, calls `i18n::set_product_name("Compilo PDF")`, and, if `theme_preference` is still the default `System`, sets it to `ThemePreference::Neon` (call it **before** `restore`, so a saved choice wins). `pub web_profile: bool`. `web_profile::NAME: &str = "Compilo PDF"`. `web_profile::CREDIT: &str = "Compilo PDF is based on PdfCraft by the ArtCraft team"`.
- Produces (i18n): `pub fn set_product_name(name: &'static str)` stores the name in a `thread_local!` (the browser build is single-threaded; native tests on other threads are unaffected). `fn brand(text: &str) -> &str` returns `text` unchanged when no name is set or `text` has no "PdfCraft"; otherwise a memoized, leaked `&'static str` with every "PdfCraft" replaced (a bounded set: one entry per distinct catalogue string; lock poisoning cannot occur in a thread-local `RefCell`, but use `try_borrow_mut` and fall back to the original text, never panic). Pure helper for tests: `fn rename(text: &str, name: &str) -> String`.
- Consumes: `ThemePreference::Neon` (Task 1).

- [ ] **Step 1: Write failing tests** in `tests/it/web_profile.rs`:
  - `web_profile_defaults_to_neon`: new app → `set_web_profile()` → `theme == Neon`.
  - `a_saved_choice_beats_the_web_default`: `set_web_profile()` then `restore(json_with_theme_Dark)` → `Dark`.
  - `home_has_no_artcraft_mark_or_community_card_on_web`: render Home with `web_profile` on; the kittest tree has no node labelled "Join the ArtCraft community" and the Help menu list (`menus::HELP` items) has none of `help.discord`, `help.website`, `help.app_page`.
  - `about_has_credit_text_and_no_artcraft_image_on_web`: open About; a node with text `web_profile::CREDIT` exists; the painted shapes contain no image whose texture was loaded from `docs/brand/` (assert `widgets::artcraft_mark`/`artcraft_logo` were not called: make them return early when the app flag is set and test through a counter, or assert the Help/About node tree).
  - `desktop_home_still_shows_the_community_card`: default app → card present (guards the desktop build).
  - `web_home_reads_compilo_pdf_not_pdfcraft`: with the profile on, Home's heading node is "Welcome to Compilo PDF" and no visible node outside the credit contains "PdfCraft"; the window title helper returns "Compilo PDF" or "<name> — Compilo PDF".
  In `i18n/tests.rs`: `rename_replaces_every_pdfcraft` (`rename("PdfCraft and PdfCraft", "Compilo PDF") == "Compilo PDF and Compilo PDF"`); `brand_is_a_noop_without_a_name` (`tr(EN, "Welcome to PdfCraft") == "Welcome to PdfCraft"`); `brand_renames_translations_once` (after `set_product_name`, `tr(ar, "About PdfCraft")` contains "Compilo PDF" and not "PdfCraft", and two calls return the same pointer); `brand_leaves_other_strings_alone` (a string without the name comes back as the same `&str`, no allocation).
- [ ] **Step 2: Run to verify failure:** `cargo test -p pdfcraft-ui-egui -j 2 --test it web_profile` → FAIL.
- [ ] **Step 3: Implement** the name hook first (it is what most tests exercise): `brand` wraps the three `tr*` functions and `t()`; `menus::APP_NAME`, the window title and the About heading read the name through a helper `web_profile::product_name(&self) -> &'static str` (`"Compilo PDF"` on the web profile, `"PdfCraft"` otherwise). Then the flag; gate each ArtCraft use on `!app.web_profile` (pass a `bool` to the two `widgets::` mark helpers or check at the call sites; do not edit the helpers' drawing code). In About replace the mark/logo block with the credit text. `grep -rn -i artcraft crates/ui-egui/src --include=*.rs` afterwards and confirm every remaining hit is a catalogue string, a comment, or behind `!web_profile`.
- [ ] **Step 4: Run to verify pass,** and `cargo check -p pdfcraft-web --target wasm32-unknown-unknown`.
- [ ] **Step 5: Wire `apps/pdfcraft-web/src/main.rs`:** call `app.set_web_profile()` right after `PdfCraftApp::new()` and before the `restore` line. Gates, commit `feat(web): web profile with Neon default and no ArtCraft marks`.

---

### Task 4: Web Home "B": drop zone and eight tool tiles

**Files:**
- Create: `crates/ui-egui/src/home/web.rs`
- Modify: `crates/ui-egui/src/home.rs` (`show` delegates to `web::show` when `app.web_profile`; declare `pub mod web;`), `crates/ui-egui/src/app/commands.rs:61` (`select_tool`, only if the tool-before-file test below fails).
- Test: `crates/ui-egui/tests/it/web_home.rs` (register like the others).

**Interfaces:**
- Produces: `pub const WEB_TILES: [(&str, &str); 8]` in `home/web.rs`, `(catalog group id, label)` in this order: `("edit","Edit")`, `("organize","Organize")`, `("comment","Comment")`, `("fill_sign","Sign")`, `("form","Forms")`, `("redact","Redact")`, `("scan","OCR")`, `("export","Convert")`; `pub fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui)`.
- Consumes: `app.web_profile` (Task 3); `catalog::group(id)`; `PdfCraftApp::select_tool(&'static str)`; `neon::glow` (Task 2).

- [ ] **Step 1: Write failing tests:**
  - `web_home_shows_the_drop_zone_above_eight_tiles_in_order`: drop zone node ("Drop a PDF here or click to open") is above the first tile; tile labels in `WEB_TILES` order.
  - `every_tile_id_exists_in_the_catalog`: `WEB_TILES` ids all satisfy `catalog::group(id).is_some()`.
  - `picking_a_tool_before_a_file_starts_it_once_the_file_opens`: click tile "Sign" with no document; then `open_bytes("a.pdf", None, fixture)`; assert `left == LeftPanel::Tool("fill_sign")` and a view is active.
  - `dropping_a_pdf_opens_it`: feed `raw.dropped_files` one valid PDF → one view.
  - Review Focus 1: `dropping_a_non_pdf_reports_and_stays_home` (bytes `b"hello"`: a notification, zero views, Home still renders, no panic); `dropping_three_pdfs_opens_three_tabs`.
  - Review Focus 2: `a_failed_open_clears_the_picked_tool`: pick "Redact", open invalid bytes → `left` is no longer `Tool("redact")`.
  - Review Focus 4: `home_fits_a_360_wide_phone`: layout at 360×640; every tile rect and the drop-zone rect lie inside the screen rect, tiles don't overlap.
  Use the fixture helper the `open_worker` tests use for a small valid PDF.
- [ ] **Step 2: Run to verify failure:** `cargo test -p pdfcraft-ui-egui -j 2 --test it web_home` → FAIL.
- [ ] **Step 3: Implement** `web::show`: title and one-line privacy sentence, a dashed magenta drop zone (egui paints dashes with `Shape::dashed_line`; `neon::glow` behind it) that opens `app.open_dialog()` on click, a wrapping grid of tiles (each with the group's `hue` icon, glowing border cycling cyan/magenta/violet/lime, `select_tool` on click), then the existing Recent list (call the shared part of `home::show`; extract the Recent block into a `fn recent(app, ui, t)` instead of copying it). The drop itself is already handled in `app.rs:441` (`open_dropped`); do not add a second handler. If `picking_a_tool_before_a_file…` fails, make `select_tool` leave `left` set (it already assigns it) and fix whatever clears it on open; if `a_failed_open_clears_the_picked_tool` fails, clear `left` back to `AllTools` in the error branch at `app.rs:~455`.
- [ ] **Step 4: Run to verify pass,** then a screenshot: `shot -- web-home.png --web-profile true --size 1440x900` and `--size 360x640`; read both PNGs.
- [ ] **Step 5: Gates, commit** `feat(ui-egui): neon web Home with drop zone and tool grid`.

---

### Task 5: Desktop-only markers

The wasm build lacks OCR, crash recovery and PKCS #12 signing (`docs/hardware-parity.md`, Web row).

**Files:**
- Modify: `crates/ui-egui/src/web_profile.rs`, the menu and panel code that exposes those three features (find by `grep -rn "ocr\|pkcs\|recover" crates/ui-egui/src crates/engine/src/commands.rs`), `docs/hardware-parity.md`.
- Test: `crates/ui-egui/tests/it/web_profile.rs`.

**Interfaces:**
- Produces: `web_profile::DESKTOP_ONLY: &[&str]` — command ids that don't run on the web; `web_profile::DOWNLOAD_URL: &str = "https://github.com/storytold/pdfcraft/releases"`; `PdfCraftApp::desktop_only(&self, command: &str) -> bool` (`web_profile && DESKTOP_ONLY.contains(&command)`).

- [ ] **Step 1: Write failing tests:** `every_desktop_only_id_is_a_real_command` (each id in `DESKTOP_ONLY` exists in `pdfcraft_engine::commands`, so a rename can't silently drop a marker); `desktop_only_items_are_disabled_with_a_download_hint_on_web` (the menu entry or tool button for each is disabled and its hover text contains "Desktop app only"); `desktop_build_is_unaffected` (`desktop_only()` is `false` without the flag for every id).
- [ ] **Step 2: Run to verify failure,** then **Step 3: implement** the list (the builder fills it from the grep; each of the three features contributes at least one id), the disabled state plus tooltip "Desktop app only" and a link "Get the PdfCraft desktop app" to `DOWNLOAD_URL` (the desktop app keeps its upstream name; this is the one other place besides the credit where "PdfCraft" appears, so `brand` must not rewrite it: build that string outside the catalogue) in the tool panel header. Update the Web row in `docs/hardware-parity.md`: "hosted deployment" leaves the gap list; revision-history line added.
- [ ] **Step 4: Run to verify pass;** **Step 5: gates, commit** `feat(ui-egui): mark desktop-only tools on the web`.

---

### Task 6: Page shell, no-network check and WebGL failure message

**Files:**
- Modify: `apps/pdfcraft-web/index.html` (neon background `#0b0716`, `<title>Compilo PDF</title>`, a visible "Loading…" line and a `<div id="pdfcraft-fallback" hidden>`, `<noscript>` text, no external URLs), `apps/pdfcraft-web/src/main.rs` (on the existing `if let Err(e) = started` path, un-hide `#pdfcraft-fallback` and fill it with web-sys; text from a pure `fn failure_text(error: &str) -> String`).
- Create: `xtask/src/web_check.rs`; modify `xtask/src/main.rs` (one table row `("web-check", …)`).
- Test: unit tests in `xtask/src/web_check.rs` and in `apps/pdfcraft-web/src/main.rs` (`failure_text` is not `cfg(wasm32)`).

**Interfaces:**
- Produces: `cargo xtask web-check [dist-dir]` (default `dist/web`) failing when `index.html` has an absolute `src`/`href`/`url(`, or when the generated `.js` contains an absolute `http(s)://` URL outside the allowlist of XML namespace strings; `fn failure_text(error: &str) -> String` in `main.rs`.

- [ ] **Step 1: Write failing tests:** in `web_check.rs`: `rejects_an_absolute_script_src`, `rejects_a_google_fonts_link`, `accepts_relative_assets_and_namespace_strings` (each as a string fed to `check_index(&str)` / `check_glue(&str)` returning `Result<(), Vec<String>>`); in `main.rs`: `failure_text_mentions_webgl_and_never_empty` (for `""`, `"WebGL not supported"`, a 10 000-char string → non-empty, ≤ 400 chars, contains "WebGL").
- [ ] **Step 2: Run to verify failure:** `cargo test -p xtask -j 2 web_check` and `cargo test -p pdfcraft-web -j 2 failure_text` → FAIL.
- [ ] **Step 3: Implement** both; run `trunk build --release` then `cargo xtask web-check`. The first run lists any absolute URLs in the glue: add each to the allowlist only if it is a namespace string, never a fetch target; anything else is a real finding to fix.
- [ ] **Step 4: Verify in a browser:** `trunk serve`, open the page, confirm neon background while loading and the Home after. Open it with WebGL disabled (Chrome flag `--disable-gpu --disable-3d-apis` on a throwaway profile) and confirm the fallback text shows, not a blank canvas.
- [ ] **Step 5: Gates, commit** `feat(web): neon page shell, offline check, WebGL failure message`.

---

### Task 7: Pages workflow and fork README (fork-only branch)

**Files:**
- Create: `.github/workflows/pages.yml`
- Modify: `README.md` (a short "Compilo PDF on the web" section at the top: live URL, the credit "Compilo PDF is based on PdfCraft by the ArtCraft team", licence pointers to `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`), `ROADMAP.md` (progress line).
- Branch: `chore/pages-fork-deploy`, from `feat/neon-web` once Tasks 1-6 are done. Never opened as a PR upstream.

**Interfaces:** consumes `trunk build --release` and `cargo xtask web-check` (Task 6).

- [ ] **Step 1: Write the workflow.** Triggers: `push` to `main` and `workflow_dispatch`. Top-level `permissions: contents: read`; the deploy job adds `pages: write` and `id-token: write`. Every job has `if: github.repository == 'St3fez/pdfcraft'`. Steps: checkout, Rust stable plus the `wasm32-unknown-unknown` target, a pinned Trunk (`cargo install --locked trunk --version <the version used in Task 0>`, cached), `cd apps/pdfcraft-web && trunk build --release`, `cargo xtask web-check`, `actions/upload-pages-artifact` on `dist/web`, `actions/deploy-pages`. Pin every action to a commit SHA with the tag in a trailing comment; resolve each with `gh api repos/<owner>/<repo>/git/ref/tags/<tag> --jq .object.sha` (annotated tags need a second hop through `git/tags/<sha>`).
- [ ] **Step 2: Lint it locally:** the repo's `packaging-lint.yml` shows the linter in use; run the same tool on `pages.yml` (or, failing that, `python -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/pages.yml`) and confirm no error. Confirm with `grep -n "uses:" .github/workflows/pages.yml` that every line carries a 40-hex SHA.
- [ ] **Step 3: README and ROADMAP.** Keep both short and factual; no ArtCraft logo or wordmark.
- [ ] **Step 4: Commit** `chore(pages): fork-only deploy workflow and web README`.

---

### Task 8: Publish and verify live (needs the owner's go-ahead)

Outward-facing and hard to reverse (a public repository). Ask the owner before Step 1, naming the exact repo (`St3fez/compilo-pdf-web`, public) and URL (`https://st3fez.github.io/compilo-pdf-web/`).

- [ ] **Step 1: Create the fork:** `gh repo fork storytold/pdfcraft --clone=false --fork-name compilo-pdf-web`; add it as remote `fork`. Set the repo description to "Compilo PDF on the web — based on PdfCraft by the ArtCraft team".
- [ ] **Step 2: Put the work on the fork's `main`:** push `feat/neon-web` and `chore/pages-fork-deploy` to `fork`, then merge both into the fork's `main` (`gh api` or a local merge pushed to `fork main`). This is the fork's own `main`, not `storytold/pdfcraft`.
- [ ] **Step 3: Enable Pages from Actions:** `gh api -X POST repos/St3fez/compilo-pdf-web/pages -f build_type=workflow`. Watch the run: `gh run watch`. Expected: green, and `gh api repos/St3fez/compilo-pdf-web/pages --jq .html_url` prints `https://st3fez.github.io/compilo-pdf-web/`.
- [ ] **Step 4: Verify in a real browser** (Chrome tools, new tab): the URL opens the neon B Home; drag-and-drop and Open file both load a synthetic PDF; a tile picked first is active after the file opens; screenshots of Home, the editor, a side panel, a menu and a dialog read as one neon system. `read_network_requests`: every request is same-origin. Reload with the network offline: it still loads.
- [ ] **Step 5: Measure and record:** transferred size of the `.wasm` and `.js` as served by Pages (Network tab) and time to first interaction; write both into the docs line added in Task 5/7 and the PR-style notes.
- [ ] **Step 6: Report to the owner** the live URL, the measured sizes, and what is marked desktop-only.

---

## Self-review

- **Spec coverage:** Goal 1 (URL + Actions) → Tasks 7, 8; Goal 2 (B Home) → Task 4; Goal 3 (Neon everywhere, pages white) → Tasks 1, 2; Goal 4 (all features, honest desktop-only) → Task 5; Goal 5 (sub-path) → Global Constraints + Task 6 `web-check`. Anti-goals: no network → Task 6/8; contrast → Task 1; desktop unchanged → tests in Tasks 1, 3, 5; no ArtCraft marks → Task 3. Brand credit text → Task 3; docs truth → Tasks 5, 7. Success criteria: screenshots + measured size → Task 8.
- **Known gap:** the spec's "a dialog and a menu read as neon" relies on `Visuals` plus the shadow colours from Task 1; if the Task 1 screenshots show a stock-looking dialog, add a step there instead of a new task.
- **Names:** `web_profile` (field), `set_web_profile`, `WEB_TILES`, `DESKTOP_ONLY`, `NeonAccents`, `glow` are used identically across tasks.
