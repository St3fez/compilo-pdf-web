//! The Home of the web profile: a drop zone above four tool tiles, then the Recent list. Opening a
//! file by dropping it is the app's own handler; this page only shows where to drop and what to do.

use egui::{CornerRadius, Rect, Sense, Shape, Stroke, pos2, vec2};
use pdfcraft_engine::catalog;

use super::card_lift;
use crate::icons;
use crate::theme::{self, Tokens, neon};
use crate::{LeftPanel, PdfCraftApp};

/// `(catalogue group id, label)` of each tile, in the order they are shown.
pub const WEB_TILES: [(&str, &str); 4] = [("fill_sign", "Fill & Sign"), ("edit", "Edit"), ("export", "Convert"), ("scan", "OCR")];

/// The space between tiles, in points.
const GAP: f32 = 14.0;
/// The narrowest and widest a tile gets, and its height.
const TILE_MIN: f32 = 140.0;
const TILE_MAX: f32 = 220.0;
const TILE_HEIGHT: f32 = 96.0;

/// How many tiles share a row on a page `width` points wide: as many as fit at the narrowest tile,
/// up to four, and always at least one so that a tile never leaves the screen.
fn columns(width: f32) -> usize {
    if !width.is_finite() || width < TILE_MIN {
        return 1;
    }
    (((width + GAP) / (TILE_MIN + GAP)).floor() as usize).clamp(1, 4)
}

/// The widest the store button gets, and the least it is tall (a comfortable thumb target).
const STORE_MAX_WIDTH: f32 = 520.0;
const STORE_HEIGHT: f32 = 56.0;
/// The padding inside the store button, and the room its phone icon takes with the gap after it.
const STORE_PAD: f32 = 18.0;
const STORE_ICON_ROOM: f32 = 34.0;

/// How long the store button's glow beats after the Home first shows, and the time of one beat, in seconds.
const PULSE_FOR: f32 = 10.0;
const PULSE_PERIOD: f32 = 2.0;
/// The glow once it has settled, and how far the first beat swings from it (the glow runs 0 to 1).
const GLOW_REST: f32 = 0.65;
const GLOW_SWING: f32 = 0.35;

/// The strength of the store button's glow `elapsed` seconds after the Home first showed. It beats
/// around [`GLOW_REST`] with a swing that fades to nothing over [`PULSE_FOR`] seconds, so it asks for a
/// look once and then settles instead of flashing for as long as the page is open. It does not beat
/// when the window does not `animated` (reduced motion), and an elapsed time that is not a positive
/// number gets the settled glow.
fn pulse(elapsed: f32, animated: bool) -> f32 {
    if !animated || !elapsed.is_finite() || elapsed <= 0.0 || elapsed >= PULSE_FOR {
        return GLOW_REST;
    }
    let fade = 1.0 - elapsed / PULSE_FOR;
    GLOW_REST + GLOW_SWING * fade * (std::f32::consts::TAU * elapsed / PULSE_PERIOD).sin()
}

/// Draw the web Home.
pub fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    // A phone keeps a narrower margin so the tiles get the width.
    let side: i8 = if ui.available_width() < 600.0 { 16 } else { 36 };
    // The Compilo PDF app for Android, for every visitor: a computer's can follow the link to the store
    // and install it on a phone. The button is pinned under the page, so it is on the first screen
    // whatever the Recent list holds, and the store opens in a new tab so the files open here stay open.
    // A panel has to be shown before the page it sits beside.
    egui::Panel::bottom("web-home-store")
        .resizable(false)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin { left: side, right: side, top: 14, bottom: 16 }))
        .show(ui, |ui| store_button(app, ui, &t));
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin { left: side, right: side, top: 28, bottom: 28 }).show(ui, |ui| {
            ui.label(egui::RichText::new(tl!("Welcome to PdfCraft")).font(theme::semibold(26.0)));
            ui.label(
                egui::RichText::new(tl!("An open-source PDF workbench — local, private, and scriptable."))
                    .color(t.text_muted)
                    .font(theme::regular(14.0)),
            );
            ui.label(
                egui::RichText::new(tl!("PdfCraft works offline. No telemetry, no account, and no cloud processing unless you add a provider."))
                    .color(t.text_muted)
                    .font(theme::regular(13.0)),
            );
            ui.add_space(18.0);
            drop_zone(app, ui, &t);
            ui.add_space(22.0);
            tiles(app, ui, &t);
            ui.add_space(26.0);
            super::recent(app, ui, &t);
        });
    });
}

/// The wide button under the Home that opens the Compilo PDF app's page on Google Play. It is lime on
/// the Neon theme (dark ink) and the accent colour on the others (white ink), and its glow beats for a
/// few seconds, see [`pulse`]. The label wraps on a phone and the button grows to hold it.
fn store_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let label = tl!(crate::web_profile::PLAY_STORE_LABEL);
    let (fill, ink) = t.neon.map_or((t.accent, egui::Color32::WHITE), |n| (n.lime, t.titlebar));
    let width = ui.available_width().min(STORE_MAX_WIDTH);
    // The label is laid out first, so the button is as tall as its lines.
    let text_room = (width - 2.0 * STORE_PAD - STORE_ICON_ROOM).max(40.0);
    let galley = ui.fonts_mut(|f| f.layout_job(egui::text::LayoutJob::simple(label.to_owned(), theme::semibold(15.0), ink, text_room)));
    let height = (galley.size().y + 2.0 * (STORE_PAD - 4.0)).max(STORE_HEIGHT);
    ui.vertical_centered(|ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, label));
        let press = theme::Press::track(ui, &resp);
        // Beats for the first seconds the Home is up; the clock starts the first time it is drawn.
        let (now, animated) = (ui.input(|i| i.time), crate::motion::animated(ui.ctx()));
        let first = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::unique("web-home-store-first-seen"), || now));
        let elapsed = (now - first) as f32;
        if animated && elapsed < PULSE_FOR {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        let glow = (pulse(elapsed, animated) + 0.3 * press.hover).min(1.0);
        let radius = CornerRadius::same(14);
        // The halo reaches past the panel that holds the button, so it may paint outside the panel.
        let outside = ui.painter().with_clip_rect(ui.clip_rect().expand(40.0));
        neon::glow(&outside, rect, radius, fill, glow);
        let body = press.fill(fill, fill.lerp_to_gamma(egui::Color32::WHITE, 0.2), fill.lerp_to_gamma(egui::Color32::BLACK, 0.2));
        ui.painter().rect_filled(rect, radius, body);
        // A phone and the label, centred together; a label that wraps keeps the phone beside its first line.
        let group = STORE_ICON_ROOM + galley.size().x;
        let left = rect.center().x - group * 0.5;
        let middle = rect.center().y + press.offset().y;
        paint_phone(ui.painter(), pos2(left + 9.0, middle), ink);
        ui.painter().galley(pos2(left + STORE_ICON_ROOM, middle - galley.size().y * 0.5), galley, ink);
        if theme::hand(resp).clicked() {
            app.open_url(crate::web_profile::PLAY_STORE_URL);
        }
    });
}

/// A small outlined phone: a rounded body and a bar for the home button, drawn here (no icon file).
fn paint_phone(painter: &egui::Painter, center: egui::Pos2, ink: egui::Color32) {
    let stroke = Stroke::new(2.0, ink);
    let body = Rect::from_center_size(center, vec2(16.0, 26.0));
    painter.rect_stroke(body, CornerRadius::same(4), stroke, egui::StrokeKind::Inside);
    painter.line_segment([pos2(center.x - 3.0, body.bottom() - 5.0), pos2(center.x + 3.0, body.bottom() - 5.0)], stroke);
}

/// The dashed magenta target that opens the file dialog; the window's own drop handler does the rest.
fn drop_zone(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Drop a PDF or Word file here or click to open")));
    let press = theme::Press::track(ui, &resp);
    // A file being dragged over the window lights the target up.
    let dragging = ui.input(|i| !i.raw.hovered_files.is_empty());
    let color = t.neon.map_or(t.accent, |n| n.magenta);
    let strength = if dragging { 1.0 } else { 0.3 + 0.4 * press.hover };
    let painter = ui.painter();
    if t.neon.is_some() {
        neon::glow(painter, rect, CornerRadius::same(14), color, strength);
    }
    painter.rect_filled(rect, CornerRadius::same(14), press.fill(t.card, t.hover, t.pressed));
    let inner = rect.shrink(1.5);
    let corners = [inner.left_top(), inner.right_top(), inner.right_bottom(), inner.left_bottom(), inner.left_top()];
    painter.extend(Shape::dashed_line(&corners, Stroke::new(2.0, color), 9.0, 7.0));
    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(body.center() - vec2(0.0, 24.0), vec2(36.0, 36.0)), "folder-open", 34.0, color);
    // The words wrap inside the target and stay centred: a language with longer words (Italian) would
    // otherwise run past both edges of a phone. With `Align::Center` the galley is placed by its centre.
    let mut job = egui::text::LayoutJob::simple(
        tl!("Drop a PDF or Word file here or click to open").to_owned(),
        theme::semibold(16.0),
        t.text,
        (rect.width() - 32.0).max(40.0),
    );
    job.halign = egui::Align::Center;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(pos2(body.center().x, body.center().y + 22.0 - galley.size().y * 0.5), galley, t.text);
    if theme::hand(resp).clicked() {
        app.open_dialog();
    }
}

/// The eight tiles in rows of [`columns`]. A tile picks its tool; the tool starts once a file opens.
fn tiles(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let width = ui.available_width();
    let columns = columns(width);
    let tile_w = (((width - GAP * (columns as f32 - 1.0)) / columns as f32).floor()).clamp(1.0, TILE_MAX);
    let accents = t.neon.map_or([t.accent; 4], |n| [n.cyan, n.magenta, n.violet, n.lime]);
    let spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = vec2(GAP, GAP);
    for (row, chunk) in WEB_TILES.chunks(columns).enumerate() {
        ui.horizontal(|ui| {
            for (col, (id, label)) in chunk.iter().enumerate() {
                let Some(g) = catalog::group(id) else { continue };
                let index = row * columns + col;
                let (rect, resp) = ui.allocate_exact_size(vec2(tile_w, TILE_HEIGHT), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!(label)));
                let press = theme::Press::track(ui, &resp);
                let border = accents.get(index % accents.len()).copied().unwrap_or(t.accent);
                // The tool picked before any file is open stays lit until the file opens or fails.
                let picked = app.active.is_none() && app.left == LeftPanel::Tool(g.id);
                if t.neon.is_some() {
                    neon::glow(ui.painter(), rect, CornerRadius::same(10), border, if picked { 1.0 } else { 0.25 + 0.5 * press.hover });
                } else {
                    card_lift(ui, t, rect, press);
                }
                let fill = press.fill(if picked { t.selected } else { t.card }, t.hover, t.pressed);
                ui.painter().rect(rect, CornerRadius::same(10), fill, Stroke::new(1.5, border), egui::StrokeKind::Inside);
                let body = rect.translate(press.offset());
                let hue = egui::Color32::from_rgb(g.hue[0], g.hue[1], g.hue[2]);
                icons::paint(ui, Rect::from_center_size(body.center() - vec2(0.0, 14.0), vec2(26.0, 26.0)), g.icon, 24.0, hue);
                let mut job = egui::text::LayoutJob::simple_singleline(tl!(label).to_owned(), theme::semibold(13.5), t.text);
                job.wrap = egui::text::TextWrapping { max_width: tile_w - 16.0, max_rows: 1, break_anywhere: true, ..Default::default() };
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let at = pos2(body.center().x - galley.size().x * 0.5, body.bottom() - 24.0 - galley.size().y * 0.5);
                ui.painter().galley(at, galley, t.text);
                if theme::hand(resp.on_hover_text(tl!(g.label))).clicked() {
                    app.select_tool(g.id);
                }
            }
        });
    }
    ui.spacing_mut().item_spacing = spacing;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tile_id_exists_in_the_catalog() {
        for (id, label) in WEB_TILES {
            assert!(catalog::group(id).is_some(), "{label}: no catalogue group {id}");
        }
    }

    #[test]
    fn the_tile_ids_are_all_different() {
        let mut ids: Vec<&str> = WEB_TILES.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), WEB_TILES.len());
    }

    #[test]
    fn the_glow_beats_at_first_and_settles_after_ten_seconds() {
        assert_eq!(pulse(0.0, true), GLOW_REST, "it starts from the settled glow");
        let beats: Vec<f32> = (1..40).map(|i| pulse(i as f32 * 0.25, true)).collect();
        assert!(beats.iter().any(|g| *g > GLOW_REST + 0.2), "a beat lights it up");
        assert!(beats.iter().any(|g| *g < GLOW_REST - 0.1), "and dims it");
        assert!(beats.iter().all(|g| (0.0..=1.0).contains(g)), "the glow stays in 0..=1: {beats:?}");
        // The swing fades out, so the last second is calmer than the first.
        let swing = |from: f32| (0..20).map(|i| (pulse(from + i as f32 * 0.05, true) - GLOW_REST).abs()).fold(0.0, f32::max);
        assert!(swing(9.0) < swing(0.5) * 0.5, "{} against {}", swing(9.0), swing(0.5));
        for settled in [PULSE_FOR, PULSE_FOR + 0.01, 60.0, 1.0e9, f32::INFINITY] {
            assert_eq!(pulse(settled, true), GLOW_REST, "{settled}");
        }
    }

    #[test]
    fn the_glow_does_not_beat_with_reduced_motion_or_a_clock_that_is_not_a_time() {
        for elapsed in [0.5, 1.5, 4.0] {
            assert_eq!(pulse(elapsed, false), GLOW_REST, "{elapsed}");
        }
        for odd in [-1.0, -0.001, f32::NAN, f32::NEG_INFINITY, f32::INFINITY] {
            assert_eq!(pulse(odd, true), GLOW_REST, "{odd}");
        }
    }

    #[test]
    fn columns_fit_the_width_and_never_drop_below_one() {
        // Four tiles of the narrowest width and three gaps need 4 * 140 + 3 * 14 = 602 points.
        assert_eq!(columns(1400.0), 4);
        assert_eq!(columns(602.0), 4);
        assert_eq!(columns(601.0), 3);
        assert_eq!(columns(448.0), 3);
        assert_eq!(columns(447.0), 2);
        assert_eq!(columns(294.0), 2);
        assert_eq!(columns(293.0), 1);
        assert_eq!(columns(330.0), 2);
        for odd in [0.0, -5.0, 10.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(columns(odd), 1, "{odd}");
        }
    }
}
