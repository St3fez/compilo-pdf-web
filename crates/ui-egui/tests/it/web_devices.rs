//! The web build on the screens visitors bring: phones and tablets, upright and turned. Sizes are
//! the browser's viewport in CSS pixels, which egui takes as points.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use pdfcraft_ui_egui::{LeftPanel, PdfCraftApp, QuickTool};

/// `(device, width, height)` upright.
const DEVICES: &[(&str, f32, f32)] = &[
    ("iphone-se", 375.0, 667.0),
    ("iphone-15", 393.0, 852.0),
    ("iphone-15-pro-max", 430.0, 932.0),
    ("android-360", 360.0, 780.0),
    ("pixel-7", 412.0, 915.0),
    ("ipad-mini", 744.0, 1133.0),
    ("ipad-air", 820.0, 1180.0),
    ("ipad-pro-13", 1024.0, 1366.0),
    ("android-tablet", 800.0, 1280.0),
];

fn harness(size: egui::Vec2, pdf: Option<Vec<u8>>) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(size).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_web_profile();
        app.set_option("language", "en").unwrap();
        if let Some(pdf) = pdf {
            app.open_bytes("form.pdf", None, pdf).unwrap();
        }
        app
    });
    h.run_steps(6);
    h
}

fn pdf() -> Vec<u8> {
    PdfCraftApp::new().session.create_from_text("fixture", "Name\n\nAddress\n\nDate and signature").unwrap().to_vec()
}

/// The widgets a finger can press, with their boxes.
fn pressable(h: &Harness<'static, PdfCraftApp>) -> Vec<(String, egui::Rect)> {
    use egui::accesskit::Role;
    h.query_all_by(|n| matches!(n.role(), Role::Button | Role::MenuItem | Role::Tab | Role::Link | Role::CheckBox | Role::ComboBox | Role::TextInput))
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .filter(|(_, r)| r.width() > 0.5 && r.height() > 0.5)
        .collect()
}

/// What is wrong on this screen: pressable widgets that leave it sideways, or that overlap.
fn problems(h: &Harness<'static, PdfCraftApp>, size: egui::Vec2) -> Vec<String> {
    let widgets = pressable(h);
    let mut found = Vec::new();
    for (label, r) in &widgets {
        if r.left() < -0.5 || r.right() > size.x + 0.5 {
            found.push(format!("{label:?} {r:?} leaves the screen sideways"));
        }
    }
    for (i, (a, ra)) in widgets.iter().enumerate() {
        for (b, rb) in widgets.iter().skip(i + 1) {
            if ra.shrink(1.0).intersects(rb.shrink(1.0)) && !ra.contains_rect(*rb) && !rb.contains_rect(*ra) {
                found.push(format!("{a:?} {ra:?} overlaps {b:?} {rb:?}"));
            }
        }
    }
    found
}

/// Phones upright: the narrowest screens the web build meets.
const PHONES: &[(&str, f32, f32)] =
    &[("android-360", 360.0, 780.0), ("iphone-se", 375.0, 667.0), ("iphone-15", 393.0, 852.0), ("iphone-15-pro-max", 430.0, 932.0)];

/// Every widget labelled `label` lies across the screen and does not start above it, and there is
/// one. A dialog taller than the screen scrolls down to what lies below it.
fn all_on_screen(h: &Harness<'static, PdfCraftApp>, size: egui::Vec2, label: &str, what: &str) {
    let rects: Vec<egui::Rect> = h.query_all_by_label(label).map(|n| n.rect()).collect();
    assert!(!rects.is_empty(), "{what}: no {label:?}");
    for r in rects {
        assert!(r.left() >= -0.5 && r.right() <= size.x + 0.5 && r.top() >= -0.5, "{what}: {label:?} {r:?} leaves the {size:?} screen");
    }
}

/// On a phone each dialog the web build opens fits the screen, so all of its buttons can be pressed.
#[test]
fn the_web_dialogs_fit_a_phone() {
    for &(device, w, hgt) in PHONES {
        let size = egui::vec2(w, hgt);
        let mut h = harness(size, Some(pdf()));
        for (command, labels) in [
            ("ocr.recognize", &["Recognize text", "Cancel"][..]),
            ("print.dialog", &["Print", "Cancel"][..]),
            ("sign.fill.signature", &["Apply", "Cancel", "Clear"][..]),
        ] {
            assert!(h.state_mut().execute(command), "{command}");
            h.run_steps(4);
            for label in labels {
                all_on_screen(&h, size, label, &format!("{device} {command}"));
            }
            h.state_mut().dialog = None;
            h.state_mut().quick_tool = QuickTool::Select;
            h.run_steps(2);
        }
        h.state_mut().palette_open = true;
        h.run_steps(3);
        all_on_screen(&h, size, "Search tools and commands", &format!("{device} command search"));
    }
}

/// On a phone the page gets the screen: a document opens on its page, not beside the tool list; a
/// tool pane, when open, takes the screen instead of squeezing the page; and picking a tool in it
/// hands the screen back to the page.
#[test]
fn the_page_gets_the_screen_on_a_phone() {
    for &(device, w, hgt) in PHONES {
        let size = egui::vec2(w, hgt);
        let mut h = harness(size, Some(pdf()));
        h.run_steps(2);
        let page = |h: &Harness<'static, PdfCraftApp>| h.state().views[0].viewport_rect().width();
        assert!(page(&h) >= 0.75 * w, "{device}: the page is {} wide on opening", page(&h));
        h.state_mut().left = LeftPanel::Tool("fill_sign");
        h.state_mut().left_open = true;
        h.run_steps(4);
        let row = h.get_by_label("Add text").rect();
        assert!(row.width() >= 0.7 * w, "{device}: the Fill & Sign list is squeezed to {row:?}");
        assert!(h.query_by_label("Select (V)").is_none(), "{device}: the page's quick tools float over the pane");
        h.get_by_label("Add text").click();
        h.run_steps(4);
        assert!(!h.state().left_open, "{device}: picking a tool closes the pane");
        assert!(page(&h) >= 0.75 * w, "{device}: the page is {} wide after picking a tool", page(&h));
    }
}

/// The bar under the tabs (menus, modes, search, save and print) fits phones and tablets upright:
/// nothing in it overlaps or leaves the screen.
#[test]
fn the_mode_bar_fits_phones_and_tablets() {
    use egui::accesskit::Role;
    let devices = PHONES.iter().copied().chain([("ipad-mini", 744.0, 1133.0)]);
    for (device, w, hgt) in devices {
        let size = egui::vec2(w, hgt);
        for pdf in [None, Some(pdf())] {
            let h = harness(size, pdf.clone());
            let bar: Vec<(String, egui::Rect)> = h
                .query_all_by(|n| matches!(n.role(), Role::Button | Role::MenuItem))
                .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
                .filter(|(_, r)| r.top() >= 37.0 && r.bottom() <= 88.0 && r.width() > 0.5)
                .collect();
            let what = format!("{device}, document open: {}", pdf.is_some());
            assert!(bar.iter().any(|(l, _)| l == "Find tools and commands"), "{what}: no search in {bar:?}");
            for (label, r) in &bar {
                assert!(r.left() >= -0.5 && r.right() <= w + 0.5, "{what}: {label:?} {r:?} leaves the screen");
            }
            for (i, (a, ra)) in bar.iter().enumerate() {
                for (b, rb) in bar.iter().skip(i + 1) {
                    assert!(!ra.shrink(1.0).intersects(rb.shrink(1.0)), "{what}: {a:?} {ra:?} overlaps {b:?} {rb:?}");
                }
            }
        }
    }
}

/// Look at every device: print what is wrong and, with PDFCRAFT_DEVICE_SHOTS set to a folder, save
/// a picture of each screen there.
#[test]
fn survey_the_devices() {
    let Ok(dir) = std::env::var("PDFCRAFT_DEVICE_SHOTS") else { return };
    let _gpu = crate::gpu();
    let dir = std::path::PathBuf::from(dir);
    let shot = |h: &mut Harness<'static, PdfCraftApp>, name: &str| {
        h.render().unwrap().save(dir.join(format!("{name}.png"))).unwrap();
    };
    for &(device, w, hgt) in DEVICES {
        for turned in [false, true] {
            let size = if turned { egui::vec2(hgt, w) } else { egui::vec2(w, hgt) };
            let name = format!("{device}-{}", if turned { "landscape" } else { "portrait" });
            let mut h = harness(size, None);
            println!("{name} home: {:?}", problems(&h, size));
            shot(&mut h, &format!("{name}-1-home"));
            let mut h = harness(size, Some(pdf()));
            println!("{name} document: {:?}", problems(&h, size));
            shot(&mut h, &format!("{name}-2-document"));
            h.state_mut().left = LeftPanel::Tool("fill_sign");
            h.state_mut().left_open = true;
            h.run_steps(4);
            println!("{name} fill & sign: {:?}", problems(&h, size));
            shot(&mut h, &format!("{name}-3-fill-sign"));
            h.state_mut().left_open = false;
            assert!(h.state_mut().execute("ocr.recognize"));
            h.run_steps(4);
            println!("{name} recognize text: {:?}", problems(&h, size));
            shot(&mut h, &format!("{name}-4-ocr"));
            for (command, picture) in [("print.dialog", "5-print"), ("sign.fill.signature", "6-signature")] {
                h.state_mut().dialog = None;
                h.run_steps(2);
                assert!(h.state_mut().execute(command));
                h.run_steps(4);
                shot(&mut h, &format!("{name}-{picture}"));
            }
        }
    }
}
