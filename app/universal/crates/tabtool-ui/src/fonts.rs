//! The bundled fonts and their deterministic fallback order.
//!
//! Inter is the interface font, Noto Sans SC covers Chinese, Japanese and
//! Korean, and Noto Emoji is the monochrome emoji fallback. The three are SIL
//! Open Font License releases, committed with their license texts under
//! `assets/fonts/`.
//!
//! Noto Emoji is deliberately *not* part of the configured proportional
//! fallback chain: a [`FontProvider`] adds it only after every other provider
//! missed. `eframe` installs the system font provider first, so a platform with
//! a color emoji font (Apple Color Emoji, Segoe UI Emoji, Noto Color Emoji)
//! renders color glyphs; elsewhere the bundled monochrome font keeps emoji
//! deterministic.

use std::sync::Arc;

use egui::{
    Context, FallbackRequest, FontData, FontDefinitions, FontFamily, FontId, FontInsert,
    FontPriority, FontProvider, FontTweak, InsertFontFamily, epaint::text::VariationCoords,
};

/// The interface font.
pub const INTER: &str = "Inter";
/// The semibold cut of Inter, used for titles.
pub const INTER_SEMIBOLD: &str = "Inter Semibold";
/// The Chinese, Japanese and Korean fallback.
pub const CJK: &str = "Noto Sans SC";
/// The monochrome emoji fallback.
pub const EMOJI: &str = "Noto Emoji";
/// The named family that always resolves to the semibold cut.
pub const SEMIBOLD_FAMILY: &str = "Inter Semibold";
/// The named family that always resolves to the bundled emoji font.
pub const EMOJI_FAMILY: &str = "Emoji";

static INTER_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-Variable.ttf");
static CJK_BYTES: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSC-Variable.ttf");
static EMOJI_BYTES: &[u8] = include_bytes!("../../../assets/fonts/NotoEmoji-Variable.ttf");

/// Install the bundled fonts and the monochrome emoji provider.
///
/// The proportional family starts with Inter, then Noto Sans SC, then egui's
/// own icon fallbacks. The monospace family keeps egui's Hack for code-like
/// cells and gains Noto Sans SC for CJK. The named [`EMOJI_FAMILY`] always
/// resolves to Noto Emoji.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        INTER.to_owned(),
        Arc::new(FontData::from_static(INTER_BYTES)),
    );
    fonts
        .font_data
        .insert(CJK.to_owned(), Arc::new(FontData::from_static(CJK_BYTES)));
    fonts.font_data.insert(
        EMOJI.to_owned(),
        Arc::new(FontData::from_static(EMOJI_BYTES)),
    );
    fonts.font_data.insert(
        INTER_SEMIBOLD.to_owned(),
        Arc::new(FontData::from_static(INTER_BYTES).tweak(FontTweak {
            coords: VariationCoords::new([(b"wght", 600.0)]),
            ..Default::default()
        })),
    );

    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, INTER.to_owned());
    let cjk_index = usize::from(!proportional.is_empty());
    proportional.insert(cjk_index, CJK.to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(CJK.to_owned());
    fonts.families.insert(
        FontFamily::Name(EMOJI_FAMILY.into()),
        vec![EMOJI.to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name(SEMIBOLD_FAMILY.into()),
        vec![INTER_SEMIBOLD.to_owned(), CJK.to_owned()],
    );

    ctx.set_fonts(fonts);
    ctx.add_font_provider(Arc::new(BundledEmoji));
}

/// A semibold font at `size`, for titles that are not using `TextStyle::Heading`.
pub fn semibold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD_FAMILY.into()))
}

/// Supplies Noto Emoji for emoji clusters no configured font covers.
struct BundledEmoji;

impl FontProvider for BundledEmoji {
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let first = request.cluster.chars().next()?;
        is_emoji(first).then(|| {
            FontInsert::new(
                EMOJI,
                FontData::from_static(EMOJI_BYTES),
                vec![InsertFontFamily {
                    family: request.family.clone(),
                    priority: FontPriority::Lowest,
                }],
            )
        })
    }
}

/// Whether `c` belongs to a Unicode block that Noto Emoji covers.
///
/// The list is intentionally conservative: the provider is only useful for
/// characters that would otherwise render as tofu, and a false positive costs
/// one failed lookup.
fn is_emoji(c: char) -> bool {
    matches!(
        c as u32,
        0x2139
            | 0x2190..=0x21FF
            | 0x2300..=0x23FF
            | 0x2600..=0x27BF
            | 0x2900..=0x297F
            | 0x2B00..=0x2BFF
            | 0x1F000..=0x1FAFF
    )
}
