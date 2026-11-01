//! The web profile: Neon by default, named Compilo PDF, and free of ArtCraft marks.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::theme::{ThemeKind, ThemePreference};
use pdfcraft_ui_egui::web_profile;

fn harness(web: bool) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        if web {
            app.set_web_profile();
        }
        app.set_option("language", "en").unwrap();
        app
    });
    h.run_steps(4);
    h
}

fn menu_row_exists(h: &Harness<'static, PdfCraftApp>, label: &str) -> bool {
    let label = label.to_owned();
    h.query_by(move |n| n.role() == egui::accesskit::Role::MenuItem && n.label().as_deref() == Some(label.as_str())).is_some()
}

#[test]
fn web_profile_defaults_to_neon() {
    let h = harness(true);
    assert!(h.state().web_profile);
    assert_eq!(h.state().theme_preference, ThemePreference::Neon);
    assert_eq!(h.state().theme, ThemeKind::Neon);
}

#[test]
fn a_saved_choice_beats_the_web_default() {
    let mut app = PdfCraftApp::new();
    app.set_web_profile();
    app.restore(r#"{"theme":"Dark"}"#);
    assert_eq!(app.theme_preference, ThemePreference::Dark);
}

#[test]
fn home_has_no_artcraft_mark_or_community_card_on_web() {
    let mut h = harness(true);
    assert!(h.query_by_label("Join the ArtCraft community").is_none());
    assert!(h.query_by_label("Discord").is_none(), "no Discord button in the top bar");
    crate::support::open_menu(&mut h, "Help");
    for label in ["Join the ArtCraft Discord", "ArtCraft website", "PdfCraft web page", "Compilo PDF web page"] {
        assert!(!menu_row_exists(&h, label), "{label} is still in the Help menu");
    }
    assert!(menu_row_exists(&h, "Keyboard shortcuts"), "the rest of the Help menu stays");
}

#[test]
fn desktop_help_menu_keeps_the_community_links() {
    let mut h = harness(false);
    crate::support::open_menu(&mut h, "Help");
    for label in ["Join the ArtCraft Discord", "ArtCraft website", "PdfCraft web page"] {
        assert!(menu_row_exists(&h, label), "{label} must stay on the desktop");
    }
}

#[test]
fn about_has_credit_text_and_no_artcraft_image_on_web() {
    let mut h = harness(true);
    h.state_mut().run_command("help.about");
    h.run_steps(4);
    h.get_by_label(web_profile::CREDIT);
    assert_eq!(h.query_all_by_label("ArtCraft").count(), 0, "neither the mark nor the wordmark is drawn");
    assert!(h.query_by_label("Join our Discord").is_none(), "the community buttons are not offered");
}

#[test]
fn desktop_about_still_shows_the_artcraft_marks() {
    let mut h = harness(false);
    // The Home behind the dialog draws its own mark on the community card.
    let behind = h.query_all_by_label("ArtCraft").count();
    h.state_mut().run_command("help.about");
    h.run_steps(4);
    assert_eq!(h.query_all_by_label("ArtCraft").count() - behind, 2, "the desktop About keeps the mark and the wordmark");
    assert!(h.query_by_label(web_profile::CREDIT).is_none());
}

#[test]
fn desktop_home_still_shows_the_community_card() {
    let h = harness(false);
    h.get_by_label("Join the ArtCraft community");
    h.get_by_label("Welcome to PdfCraft");
}

#[test]
fn web_home_reads_compilo_pdf_not_pdfcraft() {
    let h = harness(true);
    h.get_by_label("Welcome to Compilo PDF");
    let leaks: Vec<String> = h
        .query_all_by(|n| n.label().is_some_and(|l| l.contains("PdfCraft") && l != web_profile::CREDIT))
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert!(leaks.is_empty(), "the old name is still visible: {leaks:?}");
    assert_eq!(h.state().window_title, "Compilo PDF");
}

#[test]
fn desktop_window_title_is_unchanged() {
    assert_eq!(harness(false).state().window_title, "PdfCraft");
}
