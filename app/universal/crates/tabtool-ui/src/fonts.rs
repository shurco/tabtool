//! System font installation and script coverage checks.
//!
//! TabTool ships no fonts. The operating system's generic sans-serif, serif and
//! monospace families are installed in a fixed order, and
//! [`egui_system_font_provider::SystemFontProvider`] discovers a per-script
//! font (Chinese, Japanese, Korean, Cyrillic, Greek, emoji) when the installed
//! faces lack a glyph. Font files are memory-mapped, never copied into the
//! executable.

mod system;

use std::{fmt, sync::Arc};

use egui::{Context, FontFamily, FontId};
use egui_system_font_provider::SystemFontProvider;
use fontique::FontWeight;

use system::{SCRIPT_FONTS, SYSTEM_FAMILIES, ensure_fonts, system_font_definitions};

/// The named family used for titles and other semibold text.
///
/// The family starts with a system face of at least [`HEADING_WEIGHT`] when the
/// system provides one, then falls back to the proportional order. When no
/// semibold or bold face exists, the family is the proportional list.
pub const HEADING_FAMILY: &str = "UI Heading";

/// The minimum weight used for titles.
pub const HEADING_WEIGHT: FontWeight = FontWeight::SEMI_BOLD;

/// No system font family could be resolved.
///
/// TabTool ships no fonts, so it cannot start without the operating system's
/// font service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MissingSystemFonts;

impl fmt::Display for MissingSystemFonts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "no system font families were found. Install a desktop font package \
             (for example fonts-dejavu or fonts-noto on Linux), start the desktop \
             font service, and run TabTool again.",
        )
    }
}

impl std::error::Error for MissingSystemFonts {}

/// The UI scripts checked at startup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Script {
    /// Chinese, represented by 中.
    Chinese,
    /// Japanese, represented by あ.
    Japanese,
    /// Korean, represented by 한.
    Korean,
    /// Cyrillic, represented by Привет.
    Cyrillic,
    /// Greek, represented by Ελλάδα.
    Greek,
    /// Emoji, represented by 😀.
    Emoji,
}

impl Script {
    /// Every script, in the order the startup check reports them.
    pub const ALL: [Self; 6] = [
        Self::Chinese,
        Self::Japanese,
        Self::Korean,
        Self::Cyrillic,
        Self::Greek,
        Self::Emoji,
    ];

    /// The representative text that must have glyphs.
    pub const fn sample(self) -> &'static str {
        match self {
            Self::Chinese => "中",
            Self::Japanese => "あ",
            Self::Korean => "한",
            Self::Cyrillic => "Привет",
            Self::Greek => "Ελλάδα",
            Self::Emoji => "😀",
        }
    }

    /// The human-readable script name used in warnings.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Chinese => "Chinese",
            Self::Japanese => "Japanese",
            Self::Korean => "Korean",
            Self::Cyrillic => "Cyrillic",
            Self::Greek => "Greek",
            Self::Emoji => "emoji",
        }
    }

    /// The UI languages that need this script, or the content that does.
    pub const fn affected(self) -> &'static str {
        match self {
            Self::Chinese => "Simplified Chinese interface text",
            Self::Japanese => "Japanese interface text",
            Self::Korean => "Korean interface text",
            Self::Cyrillic => "Belarusian and Ukrainian interface text",
            Self::Greek => "Greek CSV content (no interface language uses it)",
            Self::Emoji => "emoji in CSV content (no interface language uses it)",
        }
    }
}

/// Install the system font families and the script fallback provider.
///
/// The proportional family is system sans-serif, then serif, then monospace.
/// The monospace family starts with system monospace and falls back to
/// sans-serif, then serif. [`HEADING_FAMILY`] starts with a semibold or bold
/// face, then falls back to the proportional order. Any character these faces
/// miss is resolved by the system font provider from the installed platform
/// fonts.
///
/// Returns [`MissingSystemFonts`] when no generic family resolves at all, so
/// the caller can stop with an actionable error instead of running without
/// fonts.
pub fn install_fonts(ctx: &Context) -> Result<(), MissingSystemFonts> {
    ensure_fonts(&SYSTEM_FAMILIES)?;

    // Starting the provider first enumerates system fonts on its background
    // thread while the generic families are resolved on this one.
    let provider: Arc<SystemFontProvider> = Arc::clone(&*SCRIPT_FONTS);
    ctx.set_fonts(system_font_definitions());
    ctx.add_font_provider(provider);
    Ok(())
}

/// A title font at `size`, for titles that are not using `TextStyle::Heading`.
///
/// The family keeps its name for the design tokens.
pub fn heading_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(HEADING_FAMILY.into()))
}

/// The glyphs the UI needs but the installed system fonts do not provide.
///
/// `has_glyph` reports whether the font set covers one character, which lets
/// tests inject a font set without constructing an egui [`Context`].
pub fn missing_scripts(mut has_glyph: impl FnMut(char) -> bool) -> Vec<Script> {
    Script::ALL
        .into_iter()
        .filter(|script| !script.sample().chars().all(&mut has_glyph))
        .collect()
}

/// Check the installed fonts and warn on stderr for each missing script.
///
/// Called after the first frame, when egui has built its font set. A character
/// counts as covered when either the proportional or the monospace family has
/// its glyph. The app keeps running and renders the glyphs that are available.
///
/// Stderr is the only warning channel: the desktop app has no logger, so a
/// launch from a GUI shell does not persist these lines.
pub fn warn_missing_scripts(ctx: &Context) -> Vec<Script> {
    let probes = [FontId::proportional(13.0), FontId::monospace(13.0)];
    let missing = ctx.fonts_mut(|fonts| {
        missing_scripts(|character| probes.iter().any(|font| fonts.has_glyph(font, character)))
    });

    for script in &missing {
        eprintln!(
            "tabtool: no system font covers {} (sample {:?}); affected: {}. \
             Install a font for this script or those characters will show as tofu boxes.",
            script.name(),
            script.sample(),
            script.affected()
        );
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::{MissingSystemFonts, Script, missing_scripts};

    #[test]
    fn the_missing_system_fonts_error_is_actionable() {
        let message = MissingSystemFonts.to_string();

        assert!(
            message.contains("Install a desktop font package"),
            "the error must tell the user how to fix it, got: {message}"
        );
    }

    #[test]
    fn every_script_is_reported_when_no_font_covers_it() {
        assert_eq!(missing_scripts(|_| false), Script::ALL.to_vec());
    }

    #[test]
    fn only_the_script_without_a_glyph_is_reported() {
        let missing = missing_scripts(|character| character != '한');

        assert_eq!(missing, vec![Script::Korean]);
    }

    #[test]
    fn complete_coverage_reports_nothing() {
        assert!(missing_scripts(|_| true).is_empty());
    }

    #[test]
    fn samples_exercise_their_script() {
        assert_eq!(Script::Chinese.sample(), "中");
        assert_eq!(Script::Japanese.sample(), "あ");
        assert_eq!(Script::Korean.sample(), "한");
        assert_eq!(Script::Cyrillic.sample(), "Привет");
        assert_eq!(Script::Greek.sample(), "Ελλάδα");
        assert_eq!(Script::Emoji.sample(), "😀");
    }
}
