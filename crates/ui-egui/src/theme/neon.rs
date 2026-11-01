//! The Neon theme: a near-black violet base with cyan, magenta, violet and lime accents. It lives in
//! its own file so the Light and Dark code in `theme.rs` stays as it was.

use egui::{Color32, CornerRadius, Mesh, Painter, Rect, Shadow, Shape, Stroke, Visuals, pos2, vec2};

use super::{ThemeKind, Tokens};

/// Height of the line [`underline`] draws.
const LINE: f32 = 1.5;
/// How far below the bar the glow of [`underline`] reaches.
const BLOOM: f32 = 24.0;

/// The four glow colours of the Neon theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeonAccents {
    pub cyan: Color32,
    pub magenta: Color32,
    pub violet: Color32,
    pub lime: Color32,
}

const ACCENTS: NeonAccents = NeonAccents {
    cyan: Color32::from_rgb(0x4D, 0xFC, 0xFF),
    magenta: Color32::from_rgb(0xFF, 0x2B, 0xD6),
    violet: Color32::from_rgb(0xB4, 0x6B, 0xFF),
    lime: Color32::from_rgb(0xB6, 0xFF, 0x3B),
};

/// The Neon palette. `accent` is the fill behind white text (primary buttons), so it is a deeper
/// violet than the glow colours; the bright cyan is `accent_text` and the glow.
pub(super) fn tokens() -> Tokens {
    Tokens {
        kind: ThemeKind::Neon,
        titlebar: Color32::from_rgb(0x07, 0x04, 0x0E),
        chrome: Color32::from_rgb(0x12, 0x0A, 0x24),
        panel: Color32::from_rgb(0x14, 0x0C, 0x28),
        pasteboard: Color32::from_rgb(0x0B, 0x07, 0x16),
        card: Color32::from_rgb(0x1A, 0x10, 0x33),
        border: Color32::from_rgb(0x3A, 0x2A, 0x66),
        divider: Color32::from_rgb(0x2A, 0x1D, 0x4D),
        text: Color32::from_rgb(0xF3, 0xEE, 0xFF),
        text_muted: Color32::from_rgb(0xBD, 0xB1, 0xDE),
        text_faint: Color32::from_rgb(0xA1, 0x93, 0xCC),
        icon: Color32::from_rgb(0xD9, 0xCF, 0xFA),
        hover: Color32::from_rgb(0x24, 0x16, 0x47),
        pressed: Color32::from_rgb(0x2E, 0x1D, 0x5A),
        selected: Color32::from_rgb(0x2B, 0x1A, 0x5E),
        accent: Color32::from_rgb(0x7A, 0x35, 0xE8),
        accent_text: Color32::from_rgb(0x7D, 0xF7, 0xFF),
        accent_soft: Color32::from_rgb(0x26, 0x16, 0x4F),
        field: Color32::from_rgb(0x0F, 0x08, 0x20),
        badge_new: Color32::from_rgb(0x7A, 0x35, 0xE8),
        page_shadow: Color32::from_rgba_unmultiplied(0xB4, 0x6B, 0xFF, 96),
        radius: 8,
        neon: Some(ACCENTS),
    }
}

/// Neon's touches on top of the dark visuals `apply` has built: violet outlines and glows in place
/// of black shadows, a cyan selection. Does nothing for another theme.
pub(super) fn style(v: &mut Visuals, t: &Tokens) {
    let Some(n) = t.neon else { return };
    v.window_stroke = Stroke::new(1.0, n.violet.gamma_multiply(0.55));
    v.selection.stroke = Stroke::new(1.0, n.cyan);
    v.hyperlink_color = n.cyan;
    v.window_shadow = Shadow { offset: [0, 0], blur: 32, spread: 2, color: n.violet.gamma_multiply(0.35) };
    v.popup_shadow = Shadow { offset: [0, 0], blur: 20, spread: 1, color: n.violet.gamma_multiply(0.3) };
}

/// A soft halo of `color` around `rect`, as one blurred shadow with no offset. `strength` runs from
/// 0 (nothing) to 1 (full); above 1 is clamped, and a value that is not a number, or a rectangle
/// that is not finite, paints nothing.
pub fn glow(painter: &Painter, rect: Rect, radius: impl Into<CornerRadius>, color: Color32, strength: f32) {
    if !strength.is_finite() || strength <= 0.0 || !rect.is_finite() {
        return;
    }
    let s = strength.min(1.0);
    // `as u8` saturates, and both values are small for a strength in 0..=1.
    let shadow =
        Shadow { offset: [0, 0], blur: (6.0 + 26.0 * s).round() as u8, spread: (2.0 * s).round() as u8, color: color.gamma_multiply(0.55 * s) };
    painter.add(shadow.as_shape(rect, radius));
}

/// The glowing line under the top bar: cyan, violet, then magenta along the bottom edge of `bar`,
/// with a violet bloom that spreads only downwards so it never tints the bar's own content.
pub fn underline(painter: &Painter, bar: Rect, accents: NeonAccents) {
    if !bar.is_finite() || bar.width() < 2.0 || bar.height() <= 0.0 {
        return;
    }
    let line = Rect::from_min_max(pos2(bar.left(), bar.bottom() - LINE), bar.right_bottom());
    let below = Rect::from_min_max(bar.left_bottom(), bar.right_bottom() + vec2(0.0, BLOOM));
    glow(&painter.with_clip_rect(below), line, 0, accents.violet, 0.6);
    let mut mesh = Mesh::default();
    for (x, color) in [(line.left(), accents.cyan), (line.center().x, accents.violet), (line.right(), accents.magenta)] {
        mesh.colored_vertex(pos2(x, line.top()), color);
        mesh.colored_vertex(pos2(x, line.bottom()), color);
    }
    for first in [0, 2] {
        mesh.add_triangle(first, first + 1, first + 2);
        mesh.add_triangle(first + 1, first + 2, first + 3);
    }
    painter.add(Shape::mesh(mesh));
}

#[cfg(test)]
mod glow_tests {
    use egui::epaint::ClippedShape;
    use egui::{Color32, Rect, Shape, pos2, vec2};

    use super::{NeonAccents, glow, tokens, underline};

    fn paint(add: impl FnOnce(&egui::Painter)) -> Vec<ClippedShape> {
        let ctx = egui::Context::default();
        let mut add = Some(add);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            if let Some(add) = add.take() {
                add(ui.painter());
            }
        });
        output.textures_delta.clear();
        output.shapes
    }

    /// The blur of every blurred rectangle among `shapes`, which is how a `Shadow` arrives.
    fn blurs(shapes: &[ClippedShape]) -> Vec<f32> {
        shapes.iter().filter_map(|s| if let Shape::Rect(r) = &s.shape { (r.blur_width > 0.0).then_some(r.blur_width) } else { None }).collect()
    }

    fn page() -> Rect {
        Rect::from_min_size(pos2(40.0, 40.0), vec2(300.0, 400.0))
    }

    fn accents() -> NeonAccents {
        tokens().neon.expect("Neon carries its accents")
    }

    #[test]
    fn glow_paints_nothing_for_zero_negative_or_non_finite_strength() {
        for strength in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let shapes = paint(|p| glow(p, page(), 0, accents().violet, strength));
            assert!(blurs(&shapes).is_empty(), "strength {strength}");
        }
    }

    #[test]
    fn glow_paints_nothing_for_a_rectangle_that_is_not_finite() {
        let shapes = paint(|p| glow(p, Rect::NOTHING, 0, accents().violet, 1.0));
        assert!(blurs(&shapes).is_empty());
    }

    #[test]
    fn glow_grows_with_strength_and_stops_at_one() {
        let blur = |strength: f32| {
            let b = blurs(&paint(|p| glow(p, page(), 0, accents().violet, strength)));
            assert_eq!(b.len(), 1, "one shape at strength {strength}");
            b[0]
        };
        assert!(blur(1.0) > blur(0.4), "a stronger glow is wider");
        assert!(blur(0.4) > 0.0);
        assert_eq!(blur(7.0), blur(1.0), "above 1 is clamped");
    }

    #[test]
    fn glow_keeps_the_colour_it_is_given() {
        let shapes = paint(|p| glow(p, page(), 0, accents().magenta, 1.0));
        let Some(Shape::Rect(r)) = shapes.iter().map(|s| &s.shape).find(|s| matches!(s, Shape::Rect(_))) else { panic!("no rectangle painted") };
        assert!(r.fill.r() > r.fill.g() && r.fill.b() > r.fill.g(), "magenta stays magenta: {:?}", r.fill);
        assert_ne!(r.fill, Color32::BLACK);
    }

    #[test]
    fn underline_paints_a_gradient_from_cyan_to_magenta() {
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 36.0));
        let shapes = paint(|p| underline(p, bar, accents()));
        let colours: Vec<Color32> = shapes
            .iter()
            .filter_map(|s| if let Shape::Mesh(m) = &s.shape { Some(m.vertices.iter().map(|v| v.color).collect::<Vec<_>>()) } else { None })
            .flatten()
            .collect();
        assert!(colours.contains(&accents().cyan) && colours.contains(&accents().magenta), "{colours:?}");
    }

    #[test]
    fn underline_paints_nothing_for_a_bar_that_is_empty_or_not_finite() {
        for bar in [Rect::NOTHING, Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 36.0)), Rect::from_min_size(pos2(f32::NAN, 0.0), vec2(10.0, 10.0))] {
            assert!(paint(|p| underline(p, bar, accents())).is_empty(), "{bar:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::Color32;

    use crate::theme::{ThemeKind, Tokens, contrast, readable};

    fn neon() -> Tokens {
        Tokens::for_kind(ThemeKind::Neon)
    }

    #[test]
    fn neon_text_meets_aa() {
        let t = neon();
        for (name, fg) in [("text", t.text), ("text_muted", t.text_muted), ("text_faint", t.text_faint), ("accent_text", t.accent_text)] {
            for (surface, bg) in [("titlebar", t.titlebar), ("chrome", t.chrome), ("panel", t.panel), ("card", t.card), ("pasteboard", t.pasteboard)]
            {
                let r = contrast(fg, bg);
                assert!(r >= 4.5, "{name} on {surface}: {r:.2}:1, WCAG AA needs 4.5:1");
            }
        }
        assert!(contrast(t.text, t.field) >= 4.5, "text on field");
        assert!(contrast(t.text, t.selected) >= 4.5, "text on a selected row");
        assert!(contrast(t.text, t.hover) >= 4.5, "text on a hovered row");
    }

    /// Primary buttons paint white text on `accent` (`widgets.rs`), so the fill has to carry it.
    #[test]
    fn white_text_on_the_accent_fill_meets_aa() {
        let t = neon();
        assert!(contrast(Color32::WHITE, t.accent) >= 4.5, "white on accent");
        assert!(contrast(Color32::WHITE, t.badge_new) >= 4.5, "white on the new badge");
    }

    #[test]
    fn neon_accents_are_the_spec_colors() {
        let a = neon().neon.expect("Neon carries its accents");
        assert_eq!(a.cyan, Color32::from_rgb(0x4d, 0xfc, 0xff));
        assert_eq!(a.magenta, Color32::from_rgb(0xff, 0x2b, 0xd6));
        assert_eq!(a.violet, Color32::from_rgb(0xb4, 0x6b, 0xff));
        assert_eq!(a.lime, Color32::from_rgb(0xb6, 0xff, 0x3b));
        assert_eq!(neon().pasteboard, Color32::from_rgb(0x0b, 0x07, 0x16));
    }

    #[test]
    fn light_and_dark_have_no_accents() {
        assert!(Tokens::for_kind(ThemeKind::Light).neon.is_none());
        assert!(Tokens::for_kind(ThemeKind::Dark).neon.is_none());
    }

    #[test]
    fn neon_is_dark_for_egui() {
        assert!(neon().dark());
        assert!(!Tokens::for_kind(ThemeKind::Light).dark());
    }

    #[test]
    fn a_document_colour_that_is_lost_on_neon_is_lifted() {
        let t = neon();
        let navy = Color32::from_rgb(0, 0, 128);
        assert!(contrast(readable(navy, t.panel, t.text), t.panel) >= 3.0);
    }
}
