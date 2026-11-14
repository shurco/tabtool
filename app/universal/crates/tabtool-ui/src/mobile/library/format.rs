//! Locale-short sizes and dates for the library rows.
//!
//! The interface languages use two decimal marks and three date orders; the
//! app has no ICU number or calendar formatter, so the supported locales are
//! mapped explicitly, as the status line does for digit grouping.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tabtool_core::Locale;

/// The locale's decimal separator: a comma in the continental languages.
fn decimal_separator(locale: &Locale) -> &'static str {
    match locale.id.language.as_str() {
        "de" | "es" | "it" | "fr" | "pl" | "be" | "uk" => ",",
        _ => ".",
    }
}

/// A file size in the short file form: `8 B`, `1.5 KB`, `2 MB`.
pub fn format_size(bytes: u64, locale: &Locale) -> String {
    const UNITS: [(&str, u64); 4] = [
        ("GB", 1_000_000_000),
        ("MB", 1_000_000),
        ("KB", 1_000),
        ("B", 1),
    ];
    let mut index = UNITS
        .iter()
        .position(|(_, scale)| bytes >= *scale)
        .unwrap_or(UNITS.len() - 1);
    let (mut unit, mut scale) = UNITS[index];
    let mut tenths = (u128::from(bytes) * 10 + u128::from(scale) / 2) / u128::from(scale);
    // A value that rounds to the next unit shows the next unit: 999,999
    // bytes reads "1 MB", not "1000 KB".
    while tenths >= 10_000 && index > 0 {
        index -= 1;
        (unit, scale) = UNITS[index];
        tenths = (u128::from(bytes) * 10 + u128::from(scale) / 2) / u128::from(scale);
    }
    let whole = tenths / 10;
    let remainder = tenths % 10;
    let number = if remainder == 0 {
        whole.to_string()
    } else {
        format!("{whole}{}{remainder}", decimal_separator(locale))
    };
    format!("{number} {unit}")
}

/// A modification time in the locale's short date and 24-hour time.
pub fn format_modified(modified: SystemTime, locale: &Locale) -> String {
    let seconds = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let time = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let (hours, minutes) = (time / 3_600, (time % 3_600) / 60);
    let language = locale.id.language.as_str();
    let us_english = language == "en"
        && locale
            .id
            .region
            .is_some_and(|region| region.as_str().eq_ignore_ascii_case("US"));
    let date = match language {
        "en" if us_english => format!("{month:02}/{day:02}/{year}"),
        "zh" | "ja" | "ko" => format!("{year}/{month:02}/{day:02}"),
        "de" | "pl" | "be" | "uk" => format!("{day:02}.{month:02}.{year}"),
        _ => format!("{day:02}/{month:02}/{year}"),
    };
    format!("{date}, {hours:02}:{minutes:02}")
}

/// The Gregorian date for a count of days since 1 January 1970.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn dates_convert_from_unix_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_675), (2023, 11, 14));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }
}
