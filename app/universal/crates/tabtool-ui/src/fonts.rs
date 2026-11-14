//! System font installation and script coverage checks.
//!
//! TabTool ships no fonts. The operating system's generic sans-serif, serif and
//! monospace families are installed in a fixed order, and
//! [`egui_system_font_provider::SystemFontProvider`] discovers a per-script
//! font (Chinese, Japanese, Korean, Cyrillic, Greek, emoji) when the installed
//! faces lack a glyph. Font files are memory-mapped, never copied into the
//! executable.

use std::{
    fmt,
    sync::{Arc, LazyLock},
};

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
use egui_system_font_provider::SystemFontProvider;
use fontique::{
    Collection, CollectionOptions, FontStyle, FontWeight, GenericFamily, QueryFamily, QueryStatus,
    SourceCache, SourceCacheOptions,
};

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

#[derive(Clone)]
struct FamilyFont {
    name: String,
    /// Read by the heading resolution test.
    #[cfg_attr(not(test), allow(dead_code))]
    weight: FontWeight,
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

#[derive(Default)]
struct SystemFamilies {
    sans: Option<FamilyFont>,
    serif: Option<FamilyFont>,
    mono: Option<FamilyFont>,
    heading: Option<FamilyFont>,
}

impl SystemFamilies {
    fn is_empty(&self) -> bool {
        self.sans.is_none() && self.serif.is_none() && self.mono.is_none() && self.heading.is_none()
    }
}

/// The startup guard: no resolved family means the window cannot render text.
fn ensure_fonts(families: &SystemFamilies) -> Result<(), MissingSystemFonts> {
    if families.is_empty() {
        Err(MissingSystemFonts)
    } else {
        Ok(())
    }
}

fn system_font_definitions() -> FontDefinitions {
    let families = &*SYSTEM_FAMILIES;
    let mut definitions = FontDefinitions::empty();

    let ordered = [&families.sans, &families.serif, &families.mono];
    for font in ordered.into_iter().flatten().chain(families.heading.iter()) {
        definitions
            .font_data
            .insert(font.name.clone(), Arc::new(font.data.clone()));
    }

    let proportional = family_names(ordered);
    let monospace = family_names([&families.mono, &families.sans, &families.serif]);
    let heading = heading_names(families.heading.as_ref(), &proportional);

    definitions
        .families
        .insert(FontFamily::Proportional, proportional);
    definitions
        .families
        .insert(FontFamily::Monospace, monospace);
    definitions
        .families
        .insert(FontFamily::Name(HEADING_FAMILY.into()), heading);

    definitions
}

/// The heading list: the heavier face first, then the proportional fallbacks.
///
/// Without a heavier face the heading list is exactly the proportional list.
fn heading_names(heading: Option<&FamilyFont>, proportional: &[String]) -> Vec<String> {
    let Some(heading) = heading else {
        return proportional.to_vec();
    };

    let mut names = vec![heading.name.clone()];
    names.extend(
        proportional
            .iter()
            .filter(|name| **name != heading.name)
            .cloned(),
    );
    names
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
        let weight = collection
            .family(font.family.0)
            .and_then(|family| family.fonts().get(font.family.1).map(|info| info.weight()))
            .unwrap_or(FontWeight::NORMAL);
        let name = format!("system:{generic:?}:{family_name}:{}", font.index);
        let (blob, _) = font.blob.into_raw_parts();

        Some(FamilyFont {
            name,
            weight,
            data: FontData::from_blob(blob, font.index),
        })
    };

    let sans = resolve(GenericFamily::SansSerif);
    let serif = resolve(GenericFamily::Serif);
    let mono = resolve(GenericFamily::Monospace);
    let heading = resolve_heading(&mut collection, &mut source_cache);

    SystemFamilies {
        sans,
        serif,
        mono,
        heading,
    }
}

/// The semibold-or-bolder face of the first generic family that has one.
///
/// Scanning the generic families directly (instead of a weighted query) keeps
/// the choice on the actual face weight: egui cannot apply variable-font axes,
/// so a face whose default instance is regular renders regular even when its
/// family ships a heavier static face.
fn resolve_heading(
    collection: &mut Collection,
    source_cache: &mut SourceCache,
) -> Option<FamilyFont> {
    for generic in [
        GenericFamily::SansSerif,
        GenericFamily::Serif,
        GenericFamily::Monospace,
    ] {
        let families: Vec<_> = collection.generic_families(generic).collect();
        for id in families {
            let Some(family) = collection.family(id) else {
                continue;
            };
            let Some(font) = family
                .fonts()
                .iter()
                .filter(|font| font.style() == FontStyle::Normal && font.weight() >= HEADING_WEIGHT)
                .min_by(|left, right| left.weight().value().total_cmp(&right.weight().value()))
            else {
                continue;
            };

            let Some(blob) = font.load(Some(source_cache)) else {
                continue;
            };
            let name = format!("system:{generic:?}:{}:{}", family.name(), font.index());
            let (data, _) = blob.into_raw_parts();

            return Some(FamilyFont {
                name,
                weight: font.weight(),
                data: FontData::from_blob(data, font.index()),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use egui::FontData;
    use fontique::FontWeight;

    use super::{
        FamilyFont, HEADING_WEIGHT, MissingSystemFonts, SYSTEM_FAMILIES, Script, SystemFamilies,
        ensure_fonts, heading_names, missing_scripts,
    };

    fn test_font(weight: FontWeight) -> FamilyFont {
        FamilyFont {
            name: format!("test:{}", weight.value()),
            weight,
            data: FontData::from_owned(Vec::new()),
        }
    }

    #[test]
    fn the_guard_rejects_a_resolution_without_any_family() {
        assert_eq!(
            ensure_fonts(&SystemFamilies::default()),
            Err(MissingSystemFonts)
        );
    }

    #[test]
    fn the_guard_accepts_a_resolution_with_one_family() {
        let families = SystemFamilies {
            sans: Some(test_font(FontWeight::NORMAL)),
            ..SystemFamilies::default()
        };

        assert_eq!(ensure_fonts(&families), Ok(()));
    }

    #[test]
    fn the_missing_system_fonts_error_is_actionable() {
        let message = MissingSystemFonts.to_string();

        assert!(
            message.contains("Install a desktop font package"),
            "the error must tell the user how to fix it, got: {message}"
        );
    }

    #[test]
    fn heading_names_are_the_proportional_list_without_a_heavier_face() {
        let proportional = vec!["regular".to_owned()];

        assert_eq!(heading_names(None, &proportional), proportional);
    }

    #[test]
    fn heading_names_put_the_heavier_face_first() {
        let heavy = test_font(FontWeight::BOLD);
        let proportional = vec!["regular".to_owned(), heavy.name.clone()];

        assert_eq!(
            heading_names(Some(&heavy), &proportional),
            vec![heavy.name.clone(), "regular".to_owned()]
        );
    }

    #[test]
    fn headings_use_a_heavier_face_when_the_system_provides_one() {
        let families = &*SYSTEM_FAMILIES;

        if let Some(heading) = &families.heading {
            assert!(
                heading.weight >= HEADING_WEIGHT,
                "the heading face must be semibold or bolder, got {}",
                heading.weight
            );
        } else {
            // No heavier face exists: the heading family falls back to the
            // proportional list, so no lighter face may claim to be heavier.
            assert!(
                [&families.sans, &families.serif, &families.mono]
                    .into_iter()
                    .flatten()
                    .all(|font| font.weight < HEADING_WEIGHT),
                "a semibold or bold face exists but headings did not use it"
            );
        }
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
