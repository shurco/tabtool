//! Resolution of the operating system's generic font families.

use std::sync::{Arc, LazyLock};

use egui::{FontData, FontDefinitions, FontFamily};
use egui_system_font_provider::SystemFontProvider;
use fontique::{
    Collection, CollectionOptions, FontStyle, FontWeight, GenericFamily, QueryFamily, QueryStatus,
    SourceCache, SourceCacheOptions,
};

use super::{HEADING_FAMILY, HEADING_WEIGHT, MissingSystemFonts};

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
pub(super) static SYSTEM_FAMILIES: LazyLock<SystemFamilies> =
    LazyLock::new(resolve_system_families);

/// The font provider that finds per-script fallbacks, shared across contexts.
pub(super) static SCRIPT_FONTS: LazyLock<Arc<SystemFontProvider>> =
    LazyLock::new(|| Arc::new(SystemFontProvider::new()));

#[derive(Default)]
pub(super) struct SystemFamilies {
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
pub(super) fn ensure_fonts(families: &SystemFamilies) -> Result<(), MissingSystemFonts> {
    if families.is_empty() {
        Err(MissingSystemFonts)
    } else {
        Ok(())
    }
}

pub(super) fn system_font_definitions() -> FontDefinitions {
    font_definitions(&SYSTEM_FAMILIES)
}

fn font_definitions(families: &SystemFamilies) -> FontDefinitions {
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

/// The role a face plays, which keeps its [`FontData`] key separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FaceRole {
    /// The proportional, serif and monospace families.
    Body,
    /// The heavier face used by [`HEADING_FAMILY`].
    Heading,
}

/// The `FontData` key for one face.
///
/// Heading faces live under their own namespace: a regular and a bold face can
/// share a family name and a file index when they are separate files (DejaVu,
/// Segoe UI), and a shared key would let the last insert overwrite the other
/// face.
fn face_name(role: FaceRole, generic: GenericFamily, family: &str, index: u32) -> String {
    let namespace = match role {
        FaceRole::Body => "system",
        FaceRole::Heading => "system-heading",
    };
    format!("{namespace}:{generic:?}:{family}:{index}")
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
        let name = face_name(FaceRole::Body, generic, &family_name, font.index);
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
            let name = face_name(FaceRole::Heading, generic, family.name(), font.index());
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
    use egui::{FontData, FontFamily};
    use fontique::{FontWeight, GenericFamily};

    use super::{
        FaceRole, FamilyFont, SYSTEM_FAMILIES, SystemFamilies, ensure_fonts, face_name,
        font_definitions, heading_names,
    };
    use crate::fonts::{HEADING_FAMILY, HEADING_WEIGHT, MissingSystemFonts};

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
    fn heading_faces_with_the_same_family_and_index_keep_their_own_data() {
        // DejaVu Sans and Segoe UI ship regular and bold as separate files at
        // face index 0, so only the namespace separates the two keys.
        let body = FamilyFont {
            name: face_name(FaceRole::Body, GenericFamily::SansSerif, "DejaVu Sans", 0),
            weight: FontWeight::NORMAL,
            data: FontData::from_static(b"regular"),
        };
        let heading = FamilyFont {
            name: face_name(
                FaceRole::Heading,
                GenericFamily::SansSerif,
                "DejaVu Sans",
                0,
            ),
            weight: FontWeight::BOLD,
            data: FontData::from_static(b"bold"),
        };
        let families = SystemFamilies {
            sans: Some(body),
            heading: Some(heading),
            ..SystemFamilies::default()
        };

        let definitions = font_definitions(&families);
        let proportional = &definitions.families[&FontFamily::Proportional];
        let heading = &definitions.families[&FontFamily::Name(HEADING_FAMILY.into())];

        assert_eq!(
            definitions.font_data.len(),
            2,
            "the regular and bold face must not share one FontData key"
        );
        assert_ne!(proportional[0], heading[0]);
        assert_eq!(
            definitions.font_data[&proportional[0]].bytes(),
            b"regular",
            "the body family must keep the regular face"
        );
        assert_eq!(
            definitions.font_data[&heading[0]].bytes(),
            b"bold",
            "the heading family must use the bold face"
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
}
