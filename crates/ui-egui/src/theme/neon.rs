//! The Neon theme: a near-black violet base with cyan, magenta, violet and lime accents. It lives in
//! its own file so the Light and Dark code in `theme.rs` stays as it was.

use egui::{Color32, Shadow, Stroke, Visuals};

use super::{ThemeKind, Tokens};

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
