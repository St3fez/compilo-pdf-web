# PdfCraft on GitHub Pages, neon edition: design

> **Status:** approved by the owner 2026-11-01 (plan: `docs/superpowers/plans/2026-11-01-pdfcraft-neon-web.md`) · **Written:** 2026-10-30 · **Owner:** St3fez (fork owner)
> **Name (decided 2026-11-01):** the published tool is called **Compilo PDF**, the same name as the owner's Android app. "PdfCraft" is not shown as the product name; it appears only in the plain-text credit.
> Marked **[call]** = a decision the brainstorm left to the writer, not something the owner said. Easy to flip.

## Intent

Put the real PdfCraft editor on the web as a public site on the owner's own GitHub fork, so anyone can
open a PDF in the browser and use the app's tools, with a modern neon look that is the same
everywhere on the site. The editor is the existing `apps/pdfcraft-web` (egui compiled to WASM, built
with Trunk); this project is the hosting, the look, and the front door.

Why it matters to the owner: a link they can share that shows the app working, without installing
anything, in a visual identity of their own.

## What the owner decided

- Publish with **GitHub Pages**, from the owner's **own fork** (not `storytold/pdfcraft`).
- Loading a PDF works **as in the app** (Open file and drag-and-drop).
- The first screen is **option B**: a drop zone at the top and a grid of tools below
  (Edit, Organize, Comment, Sign, Forms, Redact, OCR, Convert). Picking a tool before loading a file
  starts that tool once the file is open.
- **Neon, consistent across the whole site**: Home, the editor, panels, menus and dialogs.
- Aim: **all the app's features** on the site.

## Goals

1. A public site at `https://st3fez.github.io/compilo-pdf-web/` **[call: fork named `compilo-pdf-web`, because `St3fez/compilo-pdf` is already the Android app's site repo]**,
   rebuilt and redeployed automatically by a GitHub Actions workflow in the fork.
2. The web build's Home is the B layout above, restyled from the existing Home view, not a second UI.
3. A Neon theme applied to everything egui draws, with documents themselves never recoloured (pages stay
   white, with a soft glow around them).
4. Every feature the wasm build can run today is reachable from the site.
5. The site works from a sub-path (`/compilo-pdf-web/`): all URLs relative, as `Trunk.toml` already sets.
6. The product reads "Compilo PDF" everywhere a visitor sees a name: page title, window title, Home, menus, About, messages. A web-only hook in `i18n` swaps the name in catalogue strings; the desktop build is untouched.

## Non-goals

- Building features the wasm target lacks. The docs (`docs/hardware-parity.md`, Web row) list OCR,
  crash recovery and PKCS #12 signing as missing on the web. They are not part of this project.
  **[call]** They stay visible in the UI but marked as desktop-only, with a link to download the app,
  so "all features" is honest instead of broken buttons.
- Syncing the fork with upstream automatically. **[call]** A manual `git merge upstream/main` is enough.
- A custom domain, analytics, accounts, or any server-side code.
- Embedding the Japanese fonts (`CRAFT_FONTS_DIR`) in the Pages build. The code must already work with
  that input empty.
- Proposing any of this back to `storytold/pdfcraft`.

## Anti-goals (a failure even if it technically works)

- A PDF leaving the visitor's browser. No upload, no analytics, no trackers, no runtime requests to
  CDNs or font hosts (`CLAUDE.md`: no new outbound network). The site must keep working offline once loaded.
- Glow that hurts reading. Neon is decoration. Body and control text keep WCAG AA contrast.
- Any change to the desktop app's default look or behaviour.
- The ArtCraft name, wordmark or mark on the published site (see Constraints).
- Pushing to `storytold/pdfcraft` or opening a PR there.

## Constraints

**From `CLAUDE.md` and `AGENTS.md`** (these override this document):
- Rust only; no handwritten JS/TS. `index.html` may carry CSS. Generated glue is fine.
- Never panic on any input; layering (the theme lives in `ui-egui`, L7); `unsafe` forbidden.
- No font files committed. No assets from Adobe. Any new asset needs an `ATTRIBUTION.toml` entry
  (`cargo xtask assets` enforces it).
- Branch from an up-to-date `main` as `<type>/<slug>`, one task per commit, commit only green states, PR
  using `.github/pull_request_template.md`. Gates before each commit: `cargo fmt --check`, clippy
  `-D warnings`, tests, `cargo xtask layers`, `cargo xtask assets`, `cargo xtask wasm`.
  **Scope builds to the crates touched** (`-p pdfcraft-ui-egui -j 2`, under a `timeout`); a full
  workspace build has locked up this machine class.
- Keep docs true: add the ROADMAP.md progress line, update the Web row in `docs/hardware-parity.md`
  ("hosted deployment" stops being a gap), and update `parity/` only if a status becomes untrue.
- Workflows in this repo pin actions by commit SHA; the new one does too.

**Brand licence (`docs/brand/LICENSE-brand.txt`).** A modified version that is published must remove the
ArtCraft Marks or replace them with the owner's own, must not be called ArtCraft, and may say in plain
text that it is based on PdfCraft by the ArtCraft team. So the published build and page carry no ArtCraft
logo or mark, and the About/credits text says so in plain words. Code is MIT OR Apache-2.0, so a public
fork is allowed; keep the licence files and `NOTICE` in the repo.

This is not hypothetical: the app embeds the marks today. `crates/ui-egui/src/widgets.rs` (around lines
502-518) draws the ArtCraft wordmark and mark from `docs/brand/`, and `crates/ui-egui/src/home.rs` (around
line 42) shows "Join the ArtCraft community". The web build for Pages must replace or drop all of these
(find every use, not just these two), while the desktop build stays as it is.

**GitHub Pages.** A project site is served under `/pdfcraft/`. Pages ignores `_headers`, so caching and
compression headers are not ours to set; the `.wasm` is about 13 MB raw. Measure the real transfer size and
first-load time on Pages and report them. The workflow must run only in the owner's fork (guard on the
repository name) so merging it elsewhere does not fail.

**Fork-friendliness.** `main` upstream moves hundreds of commits in days and `ui-egui` and the i18n
catalogues are the most contested files. Keep the Neon theme in new files and touch existing ones minimally,
so merges stay cheap.

## Preconditions that need the owner

- `St3fez/compilo-pdf-web` **does not exist yet**. Creating a public fork and switching Pages to "GitHub Actions"
  are outward-facing; do them only with the owner's go-ahead at that step, never as a side effect.
- ~~The site's name~~ **Resolved 2026-11-01:** the owner named it **Compilo PDF**. This also settles the open
  question about keeping "PdfCraft" on a modified public site: it is not kept as the product name. The credit
  reads "Compilo PDF is based on PdfCraft by the ArtCraft team", in plain text.

## Look (the owner saw and chose this)

Dark base, neon accents: cyan `#4dfcff`, magenta `#ff2bd6`, violet `#b46bff`, lime `#b6ff3b` on a
near-black violet (`#0b0716`). Glowing dashed drop zone in magenta; tool tiles with a coloured glowing
border; a glowing underline on the top bar; the page being edited floats on a violet glow. Exact token
values, the glow technique in egui, and how much glow the WASM frame rate allows are the builder's to settle.

**[call]** Neon is a new theme available in every build and the default only in the web build, so native
tests and the headless `shot` example can render it. Light and Dark stay selectable in Preferences; they are
not removed.

## Success looks like

- The URL opens, shows the B Home in neon, and a synthetic test PDF opens by drag-and-drop and by Open file.
- A tool picked on Home is active when the file opens.
- Screenshots of Home, the editor, a side panel, a menu and a dialog all read as one neon system.
  Taken natively with `shot`, and in a real browser against the deployed site.
- Text contrast checked against AA on the Neon tokens.
- Desktop screenshots in Light and Dark unchanged.
- Gates above pass; the Pages workflow succeeds in the fork.
- The measured wasm transfer size and time to first interaction are written in the PR.

## Left to the builder

Token values and glow technique; which existing Home pieces on wasm need adding (the Home view exists, what it
shows on web is unconfirmed); how tool-before-file works on web (desktop retains chosen tools across opens);
Trunk and action versions; test split (unit for tokens and contrast, a kittest or `shot` check for Home,
a browser check for the deployed site); i18n keys for new strings and how missing keys fall back;
where the desktop-only markers hook in; README text for the fork.
