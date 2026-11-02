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
fn the_web_help_menu_has_no_updater_and_no_link_to_the_upstream_repository() {
    let mut h = harness(true);
    crate::support::open_menu(&mut h, "Help");
    for label in ["Check for updates…", "Compilo PDF on GitHub"] {
        assert!(!menu_row_exists(&h, label), "{label} is still in the Help menu: the web build updates itself and is not hosted there");
    }
}

/// A test of its own: turning the web profile on renames the product for the whole thread, so the
/// desktop's "PdfCraft on GitHub" is only itself in a thread that never did.
#[test]
fn the_desktop_help_menu_keeps_the_updater_and_the_github_link() {
    let mut h = harness(false);
    crate::support::open_menu(&mut h, "Help");
    for label in ["Check for updates…", "PdfCraft on GitHub"] {
        assert!(menu_row_exists(&h, label), "{label} must stay on the desktop");
    }
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
fn desktop_draws_no_artcraft_image_either() {
    // This fork carries neither the wordmark nor the mark (docs/brand/LICENSE-brand.txt), so the
    // desktop Home and About leave them out too: the community card and links stay as text.
    let mut h = harness(false);
    assert_eq!(h.query_all_by_label("ArtCraft").count(), 0, "the Home draws no mark");
    h.state_mut().run_command("help.about");
    h.run_steps(4);
    assert_eq!(h.query_all_by_label("ArtCraft").count(), 0, "the About dialog draws neither the mark nor the wordmark");
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

/// A small valid PDF.
fn pdf() -> Vec<u8> {
    PdfCraftApp::new().session.create_from_text("fixture", "hello").unwrap().as_slice().to_vec()
}

#[test]
fn every_desktop_only_id_is_a_real_command() {
    for id in web_profile::DESKTOP_ONLY {
        assert!(pdfcraft_engine::commands::command(id).is_some(), "{id} is not a registered command");
    }
    // OCR and certificate signing each contribute at least one id.
    assert!(web_profile::DESKTOP_ONLY.iter().any(|id| id.starts_with("ocr.")));
    assert!(web_profile::DESKTOP_ONLY.iter().any(|id| id.starts_with("sign.")));
}

#[test]
fn desktop_build_is_unaffected() {
    let app = PdfCraftApp::new();
    for id in web_profile::DESKTOP_ONLY {
        assert!(!app.desktop_only(id), "{id} must stay available on the desktop");
    }
    let mut web = PdfCraftApp::new();
    web.set_web_profile();
    assert!(web_profile::DESKTOP_ONLY.iter().all(|id| web.desktop_only(id)));
    assert!(!web.desktop_only("file.open"), "other commands are not marked");
}

#[test]
fn desktop_only_commands_are_refused_with_a_hint_on_web() {
    let mut h = harness(true);
    h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
    h.run_steps(3);
    for id in web_profile::DESKTOP_ONLY {
        h.state_mut().toast = None;
        assert!(!h.state_mut().execute(id), "{id} must not run on the web");
        let toast = h.state().toast.clone().map(|(text, _)| text).unwrap_or_default();
        assert!(toast.contains("Desktop app only"), "{id}: {toast}");
    }
}

#[test]
fn signing_with_a_certificate_is_not_listed_on_web_and_is_enabled_on_the_desktop() {
    for web in [true, false] {
        let mut h = harness(web);
        h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
        h.run_steps(3);
        crate::support::open_menu(&mut h, "Sign");
        if web {
            assert!(!menu_row_exists(&h, "Digitally sign"), "the web Sign menu lists a certificate signature it cannot make");
            crate::support::menu_row(&h, "Fill & Sign");
        } else {
            assert!(!crate::support::menu_row(&h, "Digitally sign").accesskit_node().is_disabled());
        }
    }
}

/// The tool groups the web build offers, and ones it leaves out: by the labels the lists show.
const OFFERED: [&str; 4] = ["Fill & Sign", "Edit a PDF", "Export a PDF", "Scan & OCR"];
const LEFT_OUT: [&str; 6] = ["Combine files", "Organize pages", "Add comments", "Protect a PDF", "Redact a PDF", "Prepare a form"];

/// Whether anything on screen carries `label`: a name may sit on more than one widget (the mode
/// bar has a Fill & Sign button too), so this counts instead of asking for exactly one.
fn shown(h: &Harness<'static, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by_label(label).count() > 0
}

fn shown_containing(h: &Harness<'static, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by_label_contains(label).count() > 0
}

fn with_tool_pane(web: bool, pane: pdfcraft_ui_egui::LeftPanel) -> Harness<'static, PdfCraftApp> {
    let mut h = harness(web);
    h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
    h.state_mut().left = pane;
    h.state_mut().left_open = true;
    h.run_steps(4);
    h
}

#[test]
fn the_all_tools_list_holds_only_the_four_jobs_on_web() {
    let h = with_tool_pane(true, pdfcraft_ui_egui::LeftPanel::AllTools);
    for label in OFFERED {
        assert!(shown(&h, label), "the web tool list lacks {label}");
    }
    for label in LEFT_OUT {
        assert!(!shown(&h, label), "the web tool list still shows {label}");
    }
    assert!(!shown(&h, "View more"), "four tools need no View more");
}

#[test]
fn the_desktop_all_tools_list_is_unchanged() {
    let h = with_tool_pane(false, pdfcraft_ui_egui::LeftPanel::AllTools);
    for label in OFFERED.into_iter().chain(LEFT_OUT) {
        assert!(shown(&h, label), "the desktop tool list lacks {label}");
    }
    assert!(shown(&h, "View more"));
}

#[test]
fn the_expanded_tool_view_holds_only_the_four_jobs_on_web() {
    let mut h = with_tool_pane(true, pdfcraft_ui_egui::LeftPanel::AllTools);
    h.get_by_label("Expand all tools").click();
    h.run_steps(4);
    assert!(h.state().all_tools_full, "the expanded view is showing");
    for label in OFFERED {
        assert!(shown_containing(&h, label), "the expanded web view lacks {label}");
    }
    for label in LEFT_OUT {
        assert!(!shown_containing(&h, label), "the expanded web view still shows {label}");
    }
}

#[test]
fn the_export_panel_offers_only_word_on_web() {
    for (web, others) in [(true, false), (false, true)] {
        let h = with_tool_pane(web, pdfcraft_ui_egui::LeftPanel::Tool("export"));
        assert!(shown(&h, "Microsoft Word (.docx)"), "web profile: {web}");
        assert_eq!(shown(&h, "HTML web page"), others, "web profile: {web}");
        assert_eq!(shown(&h, "Text (plain)"), others, "web profile: {web}");
    }
}

#[test]
fn the_ocr_panel_offers_only_this_file_on_web() {
    for (web, batch) in [(true, false), (false, true)] {
        let h = with_tool_pane(web, pdfcraft_ui_egui::LeftPanel::Tool("scan"));
        assert!(shown(&h, "In this file"), "web profile: {web}");
        assert_eq!(shown(&h, "In multiple files"), batch, "web profile: {web}");
        assert_eq!(shown(&h, "Enhance camera image"), batch, "web profile: {web}");
    }
}

#[test]
fn the_edit_panel_links_no_other_tool_on_web() {
    for (web, linked) in [(true, false), (false, true)] {
        let h = with_tool_pane(web, pdfcraft_ui_egui::LeftPanel::Tool("edit"));
        for label in ["Combine files", "Redact a PDF", "Prepare a form"] {
            assert_eq!(shown(&h, label), linked, "{label}; web profile: {web}");
        }
    }
}

#[test]
fn the_file_menu_keeps_open_save_and_print_and_drops_the_other_tools_on_web() {
    for web in [true, false] {
        let mut h = harness(web);
        h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
        h.run_steps(3);
        crate::support::open_menu(&mut h, "File");
        for label in ["Open…", "Save", "Print…"] {
            crate::support::menu_row(&h, label);
        }
        for label in ["Combine files…", "Create", "Protect"] {
            assert_eq!(menu_row_exists(&h, label), !web, "{label}; web profile: {web}");
        }
        crate::support::menu_row(&h, "Export");
    }
}

#[test]
fn the_command_search_finds_only_what_the_web_build_offers() {
    for (web, found) in [(true, false), (false, true)] {
        let mut h = harness(web);
        h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
        h.run_steps(3);
        // Commands of the tools the web build left out, by the exact label the palette shows. One of
        // them (Apply redactions) is in no menu and in no tool group's list, which is how it once got through.
        for (query, label) in [
            ("redaction", "Apply redactions…"),
            ("html", "Export to HTML…"),
            ("combine", "Combine files…"),
            ("multiple files", "Recognize text in multiple files…"),
        ] {
            h.state_mut().palette_open = true;
            h.state_mut().palette_query = query.to_owned();
            h.run_steps(4);
            assert_eq!(shown(&h, label), found, "search {query}; web profile: {web}");
        }
        for (query, label) in [("word", "Export to Word…"), ("fill", "Fill & Sign: add text"), ("print", "Print…"), ("recognize", "Recognize text…")]
        {
            h.state_mut().palette_open = true;
            h.state_mut().palette_query = query.to_owned();
            h.run_steps(4);
            assert!(shown(&h, label), "search {query} does not find {label}; web profile: {web}");
        }
    }
}

#[test]
fn the_ocr_panel_marks_its_desktop_only_items_and_links_to_the_download_on_web() {
    let mut h = harness(true);
    h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
    h.state_mut().left = pdfcraft_ui_egui::LeftPanel::Tool("scan");
    h.state_mut().left_open = true;
    h.run_steps(4);
    h.get_by_label(web_profile::DOWNLOAD_LABEL);
    h.get_by_label("In this file").hover();
    h.run_steps(4);
    h.get_by_label_contains("Desktop app only");
}

#[test]
fn the_desktop_ocr_panel_has_no_download_link() {
    let mut h = harness(false);
    h.state_mut().open_bytes("a.pdf", None, pdf()).unwrap();
    h.state_mut().left = pdfcraft_ui_egui::LeftPanel::Tool("scan");
    h.state_mut().left_open = true;
    h.run_steps(4);
    assert!(h.query_by_label(web_profile::DOWNLOAD_LABEL).is_none());
}
