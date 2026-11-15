//! A precise guard against hard-coded interface text.
//!
//! The scan covers the non-test Rust sources of `tabtool-ui` and
//! `tabtool-desktop`. It looks at string literals passed to the calls that
//! draw labels, buttons, tooltips, window titles and menu text, at label-like
//! struct fields, and at every literal used as a `tr` key. A literal passes
//! when it is a symbol/number/placeholder, is on the small allowlist below, is
//! a catalog key in a label-like field, or sits inside a `tr(...)` call.
//!
//! Comments and `#[cfg(test)]` modules are ignored.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tabtool_i18n::keys;

/// Text that is deliberately not translated.
///
/// Product names, URLs and the untitled document name are data, not interface
/// copy that changes with the language. Symbols, numbers and format
/// placeholders are accepted by rule below.
const ALLOWED: &[&str] = &[
    "TabTool",
    "tabtool.shurco.app",
    "https://tabtool.shurco.app",
    "https://apps.apple.com/app/id6827919069?action=write-review",
    "Untitled",
];

/// A visible call and the argument indexes that are identifiers, not copy.
const VISIBLE_CALLS: &[(&str, &[usize])] = &[
    ("ui.label(", &[]),
    ("ui.heading(", &[]),
    ("ui.button(", &[]),
    ("ui.link(", &[]),
    ("ui.small(", &[]),
    ("ui.monospace(", &[]),
    ("ui.strong(", &[]),
    ("ui.weak(", &[]),
    ("ui.selectable_label(", &[]),
    ("ui.checkbox(", &[]),
    ("RichText::new(", &[]),
    ("Label::new(", &[]),
    ("Button::new(", &[]),
    ("PushButton::new(", &[]),
    ("AlertAction::new(", &[]),
    ("MenuItem::new(", &[0]),
    ("Menu::new(", &[]),
    ("Picker::new(", &[0]),
    ("banner(", &[]),
    ("list_row(", &[]),
    ("status_line(", &[]),
    ("tab_bar(", &[]),
    ("segmented(", &[]),
    ("tool_button(", &[]),
    ("text_field(", &[]),
    ("text_field_sized(", &[]),
    ("search_field(", &[]),
    ("with_title(", &[]),
    ("accessible_name(", &[]),
    ("on_hover_text(", &[]),
    ("help(", &[]),
];

/// Label-like struct fields whose literal is normally a catalog key.
const VISIBLE_FIELDS: &[&str] = &["label", "title", "message", "hint", "tooltip"];

#[test]
fn visible_source_strings_are_catalog_keys_or_allowlisted() {
    let mut problems = Vec::new();
    for path in source_files() {
        let source = std::fs::read_to_string(&path).expect("source file is UTF-8");
        problems.extend(problems_in(&path, &source));
    }

    assert!(
        problems.is_empty(),
        "hard-coded interface text found:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_guard_catches_hard_coded_labels_and_unknown_tr_keys() {
    let path = Path::new("fixture.rs");
    let problems = problems_in(
        path,
        r#"
fn demo(ui: &mut Ui, language: Language) {
    ui.label("Hard-coded");
    ui.label(tr(language, "Cancel"));
    ui.label(tr(language, "Missing key"));
    let entry = MenuEntry { label: "Hard-coded".to_owned() };
}
"#,
    );

    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("Hard-coded"))
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("Missing key"))
    );
    assert!(
        !problems
            .iter()
            .any(|problem| problem.contains("\"Cancel\""))
    );
}

fn problems_in(path: &Path, source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let source = strip_comments(source);
    let source = source
        .split("#[cfg(test)]")
        .next()
        .expect("a split always has one part");
    let tr_spans = call_spans(source, "tr(");
    let mut seen = HashSet::new();

    for (marker, skipped) in VISIBLE_CALLS {
        for span in call_spans(source, marker) {
            for (argument, start, end) in call_arguments(source, span) {
                if skipped.contains(&argument) {
                    continue;
                }
                for literal in literals(source, start, end) {
                    check_literal(path, source, literal, &tr_spans, &mut seen, &mut problems);
                }
            }
        }
    }

    for (index, line) in source.lines().enumerate() {
        for field in VISIBLE_FIELDS {
            let Some(value) = field_literal(line, field) else {
                continue;
            };
            if keys().any(|key| key == value) || allowed(value) || is_symbol_or_placeholder(value) {
                continue;
            }
            problems.push(format!(
                "{}:{}: `{field}: \"{value}\"` is not a catalog key",
                path.display(),
                index + 1
            ));
        }
    }

    for span in tr_spans {
        let found = literals(source, span.0, span.1);
        let Some(key) = found.last() else {
            continue;
        };
        if !keys().any(|candidate| candidate == key.text) {
            problems.push(format!(
                "{}:{}: `tr` key \"{}\" is missing from the catalog",
                path.display(),
                line_at(source, key.start),
                key.text
            ));
        }
    }

    problems
}

fn check_literal(
    path: &Path,
    source: &str,
    literal: Literal<'_>,
    tr_spans: &[(usize, usize)],
    seen: &mut HashSet<usize>,
    problems: &mut Vec<String>,
) {
    if !seen.insert(literal.start) || inside(literal.start, tr_spans) {
        return;
    }
    if allowed(literal.text) || is_symbol_or_placeholder(literal.text) {
        return;
    }
    problems.push(format!(
        "{}:{}: hard-coded visible string \"{}\"",
        path.display(),
        line_at(source, literal.start),
        literal.text
    ));
}

/// Product names, URLs and the untitled document name.
fn allowed(value: &str) -> bool {
    ALLOWED.contains(&value)
}

/// Whether `value` is made only of symbols, numbers and `{}` placeholders.
fn is_symbol_or_placeholder(value: &str) -> bool {
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\\' => {
                // Skip `\u{...}` and the usual one-character escapes.
                if chars.peek() == Some(&'u') {
                    chars.next();
                    if chars.peek() == Some(&'{') {
                        for skipped in chars.by_ref() {
                            if skipped == '}' {
                                break;
                            }
                        }
                    }
                } else {
                    chars.next();
                }
            }
            '{' => {
                for skipped in chars.by_ref() {
                    if skipped == '}' {
                        break;
                    }
                }
            }
            _ if character.is_alphabetic() => return false,
            _ => {}
        }
    }
    true
}

fn source_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    collect(&root.join("crates/tabtool-ui/src"), &mut files);
    collect(&root.join("apps/tabtool-desktop/src"), &mut files);
    files
}

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("source directory is readable") {
        let entry = entry.expect("source entry is readable");
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
}

/// Removes line and block comments while preserving the line structure.
fn strip_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '"' {
            output.push(character);
            while let Some(inner) = chars.next() {
                output.push(inner);
                if inner == '\\' {
                    if let Some(escaped) = chars.next() {
                        output.push(escaped);
                    }
                } else if inner == '"' {
                    break;
                }
            }
        } else if character == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for inner in chars.by_ref() {
                if inner == '\n' {
                    output.push('\n');
                    break;
                }
            }
        } else if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut depth = 1;
            let mut previous = '\0';
            for inner in chars.by_ref() {
                if inner == '\n' {
                    output.push('\n');
                }
                if previous == '/' && inner == '*' {
                    depth += 1;
                } else if previous == '*' && inner == '/' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                previous = inner;
            }
        } else {
            output.push(character);
        }
    }
    output
}

/// The argument ranges of every occurrence of `marker` in `source`.
fn call_arguments(source: &str, span: (usize, usize)) -> Vec<(usize, usize, usize)> {
    let start = span.0 + source[span.0..].find('(').expect("the marker has `(`") + 1;
    let end = span.1.saturating_sub(1);
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut argument_start = start;
    let mut index = 0usize;
    let mut cursor = start;
    while cursor < end {
        let character = source.as_bytes()[cursor] as char;
        if character == '"' {
            cursor = string_end(source, cursor);
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                ranges.push((index, argument_start, cursor));
                index += 1;
                argument_start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    ranges.push((index, argument_start, end));
    ranges
}

/// Every `marker` call span, from the marker start through its closing `)`.
fn call_spans(source: &str, marker: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut search = 0;
    while let Some(offset) = source[search..].find(marker) {
        let start = search + offset;
        if !marker_boundary(source, start, marker) {
            search = start + 1;
            continue;
        }
        let open = start + marker.len() - 1;
        let end = paren_end(source, open);
        spans.push((start, end));
        search = start + marker.len();
    }
    spans
}

/// Rejects a marker that is only the tail of a longer identifier, e.g. `tr(`
/// inside `try_from_str(`.
fn marker_boundary(source: &str, start: usize, marker: &str) -> bool {
    if !marker.starts_with(|character: char| character.is_ascii_alphanumeric()) {
        return true;
    }
    source[..start]
        .chars()
        .next_back()
        .is_none_or(|previous| !(previous.is_ascii_alphanumeric() || previous == '_'))
}

fn paren_end(source: &str, open: usize) -> usize {
    let mut depth = 0usize;
    let mut cursor = open;
    while cursor < source.len() {
        let character = source.as_bytes()[cursor] as char;
        if character == '"' {
            cursor = string_end(source, cursor);
            continue;
        }
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return cursor + 1;
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    source.len()
}

fn string_end(source: &str, open: usize) -> usize {
    let mut cursor = open + 1;
    while cursor < source.len() {
        match source.as_bytes()[cursor] {
            b'\\' => cursor += 2,
            b'"' => return cursor + 1,
            _ => cursor += 1,
        }
    }
    source.len()
}

/// A string literal with its byte range and contents.
struct Literal<'a> {
    start: usize,
    text: &'a str,
}

fn literals(source: &str, start: usize, end: usize) -> Vec<Literal<'_>> {
    let mut literals = Vec::new();
    let mut cursor = start;
    while cursor < end {
        if source.as_bytes()[cursor] == b'"' {
            let close = string_end(source, cursor);
            literals.push(Literal {
                start: cursor,
                text: &source[cursor + 1..close - 1],
            });
            cursor = close;
        } else {
            cursor += 1;
        }
    }
    literals
}

fn field_literal<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("{field}:");
    let after = line.split_once(&needle)?.1;
    let open = after.find('"')?;
    let close = after[open + 1..].find('"')?;
    Some(&after[open + 1..open + 1 + close])
}

fn inside(position: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| *start <= position && position < *end)
}

fn line_at(source: &str, position: usize) -> usize {
    source[..position.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
