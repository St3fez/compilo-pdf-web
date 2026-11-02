//! The web Home: a drop zone above four tool tiles, whatever a visitor drops or picks.

use std::path::PathBuf;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::{LeftPanel, PdfCraftApp};

const DROP_ZONE: &str = "Drop a PDF here or click to open";
/// The tiles' labels, in the order the Home shows them.
const TILES: [&str; 4] = ["Fill & Sign", "Edit", "Convert", "OCR"];
/// The tools the web build left out: they have no tile.
const DROPPED: [&str; 4] = ["Organize", "Comment", "Forms", "Redact"];

/// A file dropped on the window, as the windowing layer hands it over.
#[derive(Debug)]
struct Dropped {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.path
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(self.bytes.clone())
    }
}

fn harness(size: egui::Vec2) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(size).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_web_profile();
        app.set_option("language", "en").unwrap();
        app
    });
    h.run_steps(4);
    h
}

fn desktop_size() -> egui::Vec2 {
    egui::vec2(1440.0, 900.0)
}

/// A small valid PDF.
fn pdf(text: &str) -> Vec<u8> {
    PdfCraftApp::new().session.create_from_text("fixture", text).unwrap().as_slice().to_vec()
}

fn drop_file(h: &mut Harness<'static, PdfCraftApp>, name: &str, bytes: Vec<u8>) {
    h.input_mut().dropped_files.push(Arc::new(Dropped { path: name.into(), bytes }));
}

/// The tile button labelled `label`: the top bar has buttons of the same names, so only one below `below`.
fn tile<'h>(h: &'h Harness<'static, PdfCraftApp>, label: &str, below: f32) -> egui_kittest::Node<'h> {
    let wanted = label.to_owned();
    h.query_all_by(move |n| n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some(wanted.as_str()))
        .find(|n| n.rect().top() >= below)
        .unwrap_or_else(|| panic!("no tile labelled {label} below y = {below}"))
}

fn zone_bottom(h: &Harness<'static, PdfCraftApp>) -> f32 {
    h.get_by_label(DROP_ZONE).rect().bottom()
}

#[test]
fn web_home_shows_the_drop_zone_above_four_tiles_in_order() {
    let h = harness(desktop_size());
    let below = zone_bottom(&h);
    let rects: Vec<egui::Rect> = TILES.iter().map(|label| tile(&h, label, below).rect()).collect();
    for pair in rects.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let next_row = b.top() >= a.bottom() - 1.0;
        let same_row_to_the_right = (b.top() - a.top()).abs() < 1.0 && b.left() >= a.right() - 1.0;
        assert!(next_row || same_row_to_the_right, "{a:?} is not followed by {b:?} in reading order");
    }
}

#[test]
fn web_home_has_no_tile_for_the_tools_it_left_out() {
    let h = harness(desktop_size());
    let below = zone_bottom(&h);
    for label in DROPPED {
        let wanted = label.to_owned();
        let found = h
            .query_all_by(move |n| n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some(wanted.as_str()))
            .any(|n| n.rect().top() >= below);
        assert!(!found, "the web Home still has a {label} tile");
    }
}

#[test]
fn picking_a_tool_before_a_file_starts_it_once_the_file_opens() {
    let mut h = harness(desktop_size());
    let below = zone_bottom(&h);
    tile(&h, "Fill & Sign", below).click();
    h.run_steps(3);
    assert!(h.state().active.is_none(), "no document is open yet");
    h.state_mut().open_bytes("a.pdf", None, pdf("a")).unwrap();
    h.run_steps(4);
    assert!(h.state().active.is_some(), "the file opened");
    assert_eq!(h.state().left, LeftPanel::Tool("fill_sign"));
}

#[test]
fn dropping_a_pdf_opens_it() {
    let mut h = harness(desktop_size());
    drop_file(&mut h, "dropped.pdf", pdf("dropped"));
    crate::support::wait_until(&mut h, "dropped document", |app| app.views.len() == 1);
    assert_eq!(h.state().views.len(), 1);
}

#[test]
fn dropping_a_non_pdf_reports_and_stays_home() {
    let mut h = harness(desktop_size());
    drop_file(&mut h, "broken.pdf", b"hello".to_vec());
    crate::support::wait_until(&mut h, "an error notification", |app| app.toast.is_some());
    h.run_steps(3);
    let toast = h.state().toast.clone().map(|(text, _)| text).unwrap_or_default();
    assert!(toast.contains("broken.pdf"), "the notification names the file: {toast}");
    assert!(h.state().views.is_empty(), "nothing opened");
    h.get_by_label(DROP_ZONE);
    h.get_by_label("Welcome to Compilo PDF");
}

#[test]
fn dropping_three_pdfs_opens_three_tabs() {
    let mut h = harness(desktop_size());
    for name in ["a.pdf", "b.pdf", "c.pdf"] {
        drop_file(&mut h, name, pdf(name));
    }
    crate::support::wait_until(&mut h, "three documents", |app| app.views.len() == 3);
    assert_eq!(h.state().views.len(), 3);
}

#[test]
fn a_failed_open_clears_the_picked_tool() {
    let mut h = harness(desktop_size());
    let below = zone_bottom(&h);
    tile(&h, "Edit", below).click();
    h.run_steps(3);
    assert_eq!(h.state().left, LeftPanel::Tool("edit"), "the tile picked the tool");
    drop_file(&mut h, "broken.pdf", b"hello".to_vec());
    crate::support::wait_until(&mut h, "an error notification", |app| app.toast.is_some());
    h.run_steps(3);
    assert_ne!(h.state().left, LeftPanel::Tool("edit"), "a later file must not start Edit");
}

#[test]
fn home_fits_a_360_wide_phone() {
    let size = egui::vec2(360.0, 640.0);
    let h = harness(size);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let zone = h.get_by_label(DROP_ZONE).rect();
    assert!(screen.contains_rect(zone), "the drop zone {zone:?} leaves the screen");
    let rects: Vec<egui::Rect> = TILES.iter().map(|label| tile(&h, label, zone.bottom()).rect()).collect();
    // The page scrolls, so the lower tiles may lie below the first screen; none may leave it sideways.
    for (label, rect) in TILES.iter().zip(&rects) {
        assert!(rect.left() >= 0.0 && rect.right() <= size.x, "the {label} tile {rect:?} leaves the screen sideways");
    }
    assert!(rects.iter().map(|r| r.top()).fold(f32::MAX, f32::min) < size.y, "the first tiles are on the first screen");
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate().skip(i + 1) {
            assert!(!a.shrink(1.0).intersects(*b), "tiles {i} and {j} overlap: {a:?} {b:?}");
        }
    }
}
