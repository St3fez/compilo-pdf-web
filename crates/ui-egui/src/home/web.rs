//! The Home of the web profile: a drop zone above four tool tiles, then the Recent list. Opening a
//! file by dropping it is the app's own handler; this page only shows where to drop and what to do.

use egui::{Align2, CornerRadius, Rect, Sense, Shape, Stroke, pos2, vec2};
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

/// Draw the web Home.
pub fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    // A phone keeps a narrower margin so the tiles get the width.
    let side: i8 = if ui.available_width() < 600.0 { 16 } else { 36 };
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

/// The dashed magenta target that opens the file dialog; the window's own drop handler does the rest.
fn drop_zone(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Drop a PDF here or click to open")));
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
    ui.painter().text(body.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, tl!("Drop a PDF here or click to open"), theme::semibold(16.0), t.text);
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
