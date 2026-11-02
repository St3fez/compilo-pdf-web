//! What the browser build changes about the app: it is called Compilo PDF, opens in the Neon theme
//! and carries none of the ArtCraft marks. The desktop build never turns the profile on.

use std::collections::HashSet;
use std::sync::OnceLock;

use pdfcraft_engine::catalog;

use crate::PdfCraftApp;
use crate::theme::ThemePreference;

/// The product name a visitor sees on the web.
pub const NAME: &str = "Compilo PDF";

/// The page's title on the web Home: the one `apps/pdfcraft-web/index.html` gives, which search
/// engines read once the app has started. A document's page names the document instead.
pub const HOME_TITLE: &str = "Compilo PDF – compila, firma e modifica PDF online gratis";

/// The one place the web build still names PdfCraft and the ArtCraft team: the credit the licences
/// ask for. It is shown as written, never through the catalogues, so it is not renamed.
pub const CREDIT: &str = "Compilo PDF is based on PdfCraft by the ArtCraft team";

/// The Help commands the web build drops: the links to ArtCraft's community, site and upstream web
/// page and to the upstream repository, and the update check, which opens the desktop releases.
pub const HIDDEN_HELP: &[&str] = &["help.discord", "help.website", "help.app_page", "help.github", "help.check_updates"];

/// The commands that do not run in the browser build: recognizing text in multiple files (it reads
/// the models from disk), enhancing a camera image, and signing with a PKCS #12 certificate
/// (docs/hardware-parity.md, Web row). Recognize text runs: it downloads its models from the site.
/// Crash recovery is the other gap, but it is an automatic start-up dialog, not a command, so it has
/// no id here.
pub const DESKTOP_ONLY: &[&str] = &["ocr.recognize_batch", "ocr.enhance_camera", "sign.digital", "sign.certify", "sign.certify_invisible"];

/// The tool groups the web build offers: filling and signing, editing, converting to Word, and OCR.
/// Printing is File ▸ Print, which no group lists. Every other group is left out of the Home, the
/// tool lists, the menus and the command search; the desktop build offers them all.
pub const GROUPS: &[&str] = &["fill_sign", "edit", "export", "scan"];

/// Inside these groups the web build offers only some of the tools: Word is its one export format,
/// and OCR reads the open file.
const ONLY: &[(&str, &[&str])] = &[("export", &["export.docx"]), ("scan", &["ocr.recognize"])];

/// Whether the web build offers the tool group `id`.
pub fn offers_group(id: &str) -> bool {
    GROUPS.contains(&id)
}

/// Whether the web build offers `command`, the tool of group `group`.
pub fn offers_tool(group: &str, command: &str) -> bool {
    offers_group(group) && ONLY.iter().find(|(g, _)| *g == group).is_none_or(|(_, only)| only.contains(&command))
}

/// Commands of the tools the web build leaves out that no tool group lists, so the groups cannot
/// name them: the permissions password (Protect) and the signatures panel (certificates).
const ALSO_HIDDEN: &[&str] = &["protect.unlock", "sign.panel"];

/// What the catalogue says about the web build, worked out once: the commands it hides and the
/// commands an offered group lists as one of its tools.
struct Rules {
    /// The commands that some tool group lists and no offered group does, [`ALSO_HIDDEN`] and
    /// [`HIDDEN_HELP`]. A command no group lists (Open, Save, Print, Undo, zoom) is never in it.
    hidden: HashSet<&'static str>,
    offered: HashSet<&'static str>,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        let (mut listed, mut offered) = (HashSet::new(), HashSet::new());
        for g in catalog::TOOL_GROUPS {
            for item in g.sections.iter().flat_map(|s| s.items) {
                for command in std::iter::once(item.command).chain(item.actions.iter().copied()) {
                    listed.insert(command);
                    if offers_tool(g.id, item.command) {
                        offered.insert(command);
                    }
                }
            }
        }
        let mut hidden: HashSet<&'static str> = listed.difference(&offered).copied().collect();
        hidden.extend(ALSO_HIDDEN);
        hidden.extend(HIDDEN_HELP);
        Rules { hidden, offered }
    })
}

/// Whether the web build offers `command`.
pub fn offers_command(command: &str) -> bool {
    !rules().hidden.contains(command)
}

/// Whether `command` is one of the tools an offered group lists.
pub fn is_offered_tool(command: &str) -> bool {
    rules().offered.contains(command)
}

/// Where the desktop app is published.
pub const DOWNLOAD_URL: &str = "https://github.com/storytold/pdfcraft/releases";

/// The link to [`DOWNLOAD_URL`]. The desktop app keeps its upstream name, so this is written here
/// and never goes through the catalogues, which the web build renames.
pub const DOWNLOAD_LABEL: &str = "Get the PdfCraft desktop app";

/// Below this width, in points, the web build lays itself out for a phone: one pane at a time,
/// taking the screen instead of squeezing the page, and closed once a tool is picked in it.
pub const PHONE_WIDTH: f32 = 600.0;

/// The store page of the Compilo PDF app for Android, which the web Home suggests on Android.
pub const PLAY_STORE_URL: &str = "https://play.google.com/store/apps/details?id=com.pdfcraft.pdfcraft_mobile";

/// The link to [`PLAY_STORE_URL`].
pub const PLAY_STORE_LABEL: &str = "On Android? Get the Compilo PDF app on Google Play";

/// What the app calls itself: [`NAME`] on the web profile, "PdfCraft" otherwise.
pub(crate) fn product_name(web_profile: bool) -> &'static str {
    if web_profile { NAME } else { "PdfCraft" }
}

impl PdfCraftApp {
    /// Turn the web profile on. Call it before `restore`, so a theme the visitor saved wins over
    /// the Neon default.
    pub fn set_web_profile(&mut self) {
        self.web_profile = true;
        #[cfg(target_arch = "wasm32")]
        {
            self.android = web_sys::window().and_then(|w| w.navigator().user_agent().ok()).is_some_and(|agent| agent.contains("Android"));
        }
        crate::i18n::set_product_name(NAME);
        if self.theme_preference == ThemePreference::System {
            self.set_theme_preference(ThemePreference::Neon);
        }
    }

    /// Whether `command` is one the web profile cannot run (see [`DESKTOP_ONLY`]).
    pub fn desktop_only(&self, command: &str) -> bool {
        self.web_profile && DESKTOP_ONLY.contains(&command)
    }

    /// Whether the web build is laid out for a phone this frame (see [`PHONE_WIDTH`]).
    pub(crate) fn phone_layout(&self) -> bool {
        self.web_profile && self.ctx.as_ref().is_some_and(|ctx| ctx.content_rect().width() < PHONE_WIDTH)
    }

    /// Whether the web build leaves the tool group `id` out of its lists (see [`GROUPS`]).
    pub(crate) fn hides_group(&self, id: &str) -> bool {
        self.web_profile && !offers_group(id)
    }

    /// Whether the web build leaves the tool `command` of group `group` out of that group's list.
    pub(crate) fn hides_tool(&self, group: &str, command: &str) -> bool {
        self.web_profile && !offers_tool(group, command)
    }

    /// Whether the web build leaves `command` out of its menus and its command search.
    pub(crate) fn hides_command(&self, command: &str) -> bool {
        self.web_profile && !offers_command(command)
    }

    /// What the title bar and the About box call the app.
    pub(crate) fn product_name(&self) -> &'static str {
        product_name(self.web_profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offered_groups_and_their_only_lists_name_real_catalogue_entries() {
        for id in GROUPS {
            assert!(catalog::group(id).is_some(), "{id} is not a catalogue group");
        }
        for (id, only) in ONLY {
            assert!(offers_group(id), "{id} has an only-list but is not offered");
            let group = catalog::group(id).expect("catalogue group");
            for command in *only {
                assert!(group.sections.iter().flat_map(|s| s.items).any(|i| i.command == *command), "{command} is not a tool of {id}");
            }
        }
    }

    #[test]
    fn the_four_jobs_are_offered() {
        for command in ["sign.fill.text", "sign.fill.signature", "edit.edit_text", "page.rotate", "export.docx", "ocr.recognize"] {
            assert!(offers_command(command), "{command} must be offered");
        }
    }

    #[test]
    fn commands_no_tool_group_lists_are_offered() {
        for command in ["file.open", "file.save", "file.save_as", "file.close", "print.dialog", "edit.undo", "edit.find", "app.preferences"] {
            assert!(offers_command(command), "{command} must be offered");
        }
    }

    #[test]
    fn the_other_tools_are_hidden() {
        for command in [
            "page.combine",
            "comment.note",
            "redact.mark",
            "protect.password",
            "protect.unlock",
            "form.fields",
            "create.file",
            "optimize.advanced",
            "export.text",
            "export.html",
            "ocr.recognize_batch",
            "sign.digital",
            "sign.panel",
            "help.check_updates",
            "help.github",
        ] {
            assert!(!offers_command(command), "{command} must be hidden");
        }
    }

    #[test]
    fn a_tool_an_offered_group_lists_is_never_hidden_because_another_group_lists_it_too() {
        for g in catalog::TOOL_GROUPS {
            for item in g.sections.iter().flat_map(|s| s.items) {
                if offers_tool(g.id, item.command) {
                    assert!(offers_command(item.command), "{} in {}", item.command, g.id);
                }
            }
        }
    }

    #[test]
    fn the_desktop_hides_nothing() {
        let app = PdfCraftApp::new();
        for g in catalog::TOOL_GROUPS {
            assert!(!app.hides_group(g.id));
            for item in g.sections.iter().flat_map(|s| s.items) {
                assert!(!app.hides_tool(g.id, item.command) && !app.hides_command(item.command), "{}", item.command);
            }
        }
    }

    #[test]
    fn the_web_profile_hides_the_groups_it_does_not_offer() {
        /// `set_web_profile` renames the product for this thread; put it back for the next test.
        struct ResetName;
        impl Drop for ResetName {
            fn drop(&mut self) {
                crate::i18n::set_product_name("");
            }
        }
        let _reset = ResetName;
        let mut app = PdfCraftApp::new();
        app.set_web_profile();
        assert!(app.hides_group("redact") && !app.hides_group("scan"));
        assert!(app.hides_tool("export", "export.html") && !app.hides_tool("export", "export.docx"));
        assert!(app.hides_command("page.combine") && !app.hides_command("print.dialog"));
    }
}
