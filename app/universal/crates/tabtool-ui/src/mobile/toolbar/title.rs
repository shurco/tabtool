//! The toolbar title: ellipsis shortening and its per-frame cache.

/// Shortens `title` with an ASCII ellipsis until `measure` fits `max_width`.
///
/// The largest fitting prefix is found by binary search, so a long name costs
/// a logarithmic number of measurements rather than one per character.
fn elide_title(title: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> String {
    if max_width <= 0.0 {
        return String::new();
    }
    if measure(title) <= max_width {
        return title.to_owned();
    }
    let characters: Vec<char> = title.chars().collect();
    let candidate = |end: usize| format!("{}...", characters[..end].iter().collect::<String>());
    let mut low = 0;
    let mut high = characters.len();
    while low < high {
        let middle = (low + high).div_ceil(2);
        if measure(&candidate(middle)) <= max_width {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    candidate(low)
}

/// The elided title, cached per title, width and font size.
///
/// The toolbar redraws every frame; measuring the same long name each time is
/// wasted work. The cache lives in the egui context, so it follows the theme
/// and is dropped with the context.
pub(super) fn cached_elided_title(
    ctx: &egui::Context,
    parent: egui::Id,
    title: &str,
    max_width: f32,
    font: egui::FontId,
    color: egui::Color32,
) -> String {
    let id = parent.with((
        "mobile-toolbar-elided-title",
        title,
        max_width.to_bits(),
        font.size.to_bits(),
    ));
    if let Some(cached) = ctx.data(|data| data.get_temp::<String>(id)) {
        return cached;
    }
    let elided = elide_title(title, max_width, |candidate| {
        ctx.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(candidate.to_owned(), font.clone(), color)
                .size()
                .x
        })
    });
    ctx.data_mut(|data| data.insert_temp(id, elided.clone()));
    elided
}

#[cfg(test)]
mod tests {
    use super::elide_title;

    #[test]
    fn a_fitting_title_is_unchanged() {
        assert_eq!(
            elide_title("Budget.csv", 100.0, |text| text.len() as f32),
            "Budget.csv"
        );
    }

    #[test]
    fn a_long_title_is_shortened_with_an_ascii_ellipsis() {
        assert_eq!(
            elide_title("a long document name.csv", 10.0, |text| text.len() as f32),
            "a long ..."
        );
    }

    #[test]
    fn a_title_wider_than_its_cell_still_ends_in_three_dots() {
        assert_eq!(elide_title("abcdef", 2.0, |text| text.len() as f32), "...");
    }

    #[test]
    fn a_long_title_uses_a_logarithmic_number_of_measurements() {
        use core::cell::Cell;

        let calls = Cell::new(0);
        let title = "a".repeat(200);
        let result = elide_title(&title, 50.0, |text| {
            calls.set(calls.get() + 1);
            text.len() as f32
        });

        assert_eq!(result, format!("{}...", "a".repeat(47)));
        assert!(
            calls.get() <= 10,
            "binary search measured {} times",
            calls.get()
        );
    }
}
