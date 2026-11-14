//! System font installation and script coverage checks.
//!
//! TabTool ships no fonts. The operating system's generic sans-serif, serif and
//! monospace families are installed in a fixed order, and
//! [`egui_system_font_provider::SystemFontProvider`] discovers a per-script
//! font (Chinese, Japanese, Korean, Cyrillic, Greek, emoji) when the installed
//! faces lack a glyph. Font files are memory-mapped, never copied into the
//! executable.

use std::sync::{Arc, LazyLock};

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
use egui_system_font_provider::SystemFontProvider;
use fontique::{
    Collection, CollectionOptions, GenericFamily, QueryFamily, QueryStatus, SourceCache,
    SourceCacheOptions,
};

/// The named family used for titles and other semibold text.
///
/// System fonts do not expose a portable weight choice through
/// [`FontData`], so the family resolves to the system sans-serif face before
/// the serif and monospace fallbacks.
pub const HEADING_FAMILY: &str = "UI Heading";

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
/// sans-serif, then serif. [`HEADING_FAMILY`] uses the proportional order.
/// Any character these faces miss is resolved by the system font provider from
/// the installed platform fonts.
pub fn install_fonts(ctx: &Context) {
    // Starting the provider first enumerates system fonts on its background
    // thread while the generic families are resolved on this one.
    let provider: Arc<SystemFontProvider> = Arc::clone(&*SCRIPT_FONTS);
    ctx.set_fonts(system_font_definitions());
    ctx.add_font_provider(provider);
}

/// A title font at `size`, for titles that are not using `TextStyle::Heading`.
///
/// The family keeps its name for the design tokens; the face is the system
/// sans-serif face.
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
/// Called after the first frame, when egui has built its font set. The app
/// keeps running and renders the glyphs that are available.
pub fn warn_missing_scripts(ctx: &Context) -> Vec<Script> {
    let font = FontId::proportional(13.0);
    let missing =
        ctx.fonts_mut(|fonts| missing_scripts(|character| fonts.has_glyph(&font, character)));

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

#[derive(Clone)]
struct FamilyFont {
    name: String,
    data: FontData,
}

/// The resolved system families, shared by every context in the process.
///
/// Enumerating system fonts is expensive, so the resolution happens once even
/// though each [`crate::UiHarness`] installs its own font definitions.
static SYSTEM_FAMILIES: LazyLock<SystemFamilies> = LazyLock::new(resolve_system_families);

/// The font provider that finds per-script fallbacks, shared across contexts.
static SCRIPT_FONTS: LazyLock<Arc<SystemFontProvider>> =
    LazyLock::new(|| Arc::new(SystemFontProvider::new()));

struct SystemFamilies {
    sans: Option<FamilyFont>,
    serif: Option<FamilyFont>,
    mono: Option<FamilyFont>,
}

fn system_font_definitions() -> FontDefinitions {
    let families = &*SYSTEM_FAMILIES;
    let mut definitions = FontDefinitions::empty();

    let ordered = [&families.sans, &families.serif, &families.mono];
    for font in ordered.into_iter().flatten() {
        definitions
            .font_data
            .insert(font.name.clone(), Arc::new(font.data.clone()));
    }

    let proportional = family_names(ordered);
    let monospace = family_names([&families.mono, &families.sans, &families.serif]);

    definitions
        .families
        .insert(FontFamily::Proportional, proportional.clone());
    definitions
        .families
        .insert(FontFamily::Monospace, monospace);
    definitions
        .families
        .insert(FontFamily::Name(HEADING_FAMILY.into()), proportional);

    definitions
}

fn family_names(fonts: [&Option<FamilyFont>; 3]) -> Vec<String> {
    fonts
        .into_iter()
        .flatten()
        .map(|font| font.name.clone())
        .collect()
}

fn resolve_system_families() -> SystemFamilies {
    let mut collection = Collection::new(CollectionOptions {
        shared: false,
        system_fonts: true,
    });
    let mut source_cache = SourceCache::new(SourceCacheOptions::default());

    let mut resolve = |generic: GenericFamily| -> Option<FamilyFont> {
        let font = {
            let mut query = collection.query(&mut source_cache);
            query.set_families([QueryFamily::Generic(generic)]);
            let mut found = None;
            query.matches_with(|candidate| {
                found = Some(candidate.clone());
                QueryStatus::Stop
            });
            found
        }?;

        let family_name = collection
            .family_name(font.family.0)
            .unwrap_or("unknown")
            .to_owned();
        let name = format!("system:{generic:?}:{family_name}:{}", font.index);
        let (blob, _) = font.blob.into_raw_parts();

        Some(FamilyFont {
            name,
            data: FontData::from_blob(blob, font.index),
        })
    };

    let sans = resolve(GenericFamily::SansSerif);
    let serif = resolve(GenericFamily::Serif);
    let mono = resolve(GenericFamily::Monospace);

    SystemFamilies { sans, serif, mono }
}

#[cfg(test)]
mod tests {
    use super::{Script, missing_scripts};

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
