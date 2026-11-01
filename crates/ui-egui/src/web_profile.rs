//! What the browser build changes about the app: it is called Compilo PDF, opens in the Neon theme
//! and carries none of the ArtCraft marks. The desktop build never turns the profile on.

use crate::PdfCraftApp;
use crate::theme::ThemePreference;

/// The product name a visitor sees on the web.
pub const NAME: &str = "Compilo PDF";

/// The one place the web build still names PdfCraft and the ArtCraft team: the credit the licences
/// ask for. It is shown as written, never through the catalogues, so it is not renamed.
pub const CREDIT: &str = "Compilo PDF is based on PdfCraft by the ArtCraft team";

/// The Help commands that point at ArtCraft's community, site or the upstream web page.
pub const HIDDEN_HELP: &[&str] = &["help.discord", "help.website", "help.app_page"];

/// The commands that do not run in the browser build: OCR and signing with a PKCS #12 certificate
/// (docs/hardware-parity.md, Web row). Crash recovery is the third gap, but it is an automatic
/// start-up dialog, not a command, so it has no id here.
pub const DESKTOP_ONLY: &[&str] =
    &["ocr.recognize", "ocr.recognize_batch", "ocr.enhance_camera", "sign.digital", "sign.certify", "sign.certify_invisible"];

/// Where the desktop app is published.
pub const DOWNLOAD_URL: &str = "https://github.com/storytold/pdfcraft/releases";

/// The link to [`DOWNLOAD_URL`]. The desktop app keeps its upstream name, so this is written here
/// and never goes through the catalogues, which the web build renames.
pub const DOWNLOAD_LABEL: &str = "Get the PdfCraft desktop app";

/// What the app calls itself: [`NAME`] on the web profile, "PdfCraft" otherwise.
pub(crate) fn product_name(web_profile: bool) -> &'static str {
    if web_profile { NAME } else { "PdfCraft" }
}

impl PdfCraftApp {
    /// Turn the web profile on. Call it before `restore`, so a theme the visitor saved wins over
    /// the Neon default.
    pub fn set_web_profile(&mut self) {
        self.web_profile = true;
        crate::i18n::set_product_name(NAME);
        if self.theme_preference == ThemePreference::System {
            self.set_theme_preference(ThemePreference::Neon);
        }
    }

    /// Whether `command` is one the web profile cannot run (see [`DESKTOP_ONLY`]).
    pub fn desktop_only(&self, command: &str) -> bool {
        self.web_profile && DESKTOP_ONLY.contains(&command)
    }

    /// What the title bar and the About box call the app.
    pub(crate) fn product_name(&self) -> &'static str {
        product_name(self.web_profile)
    }
}
