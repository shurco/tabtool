//! Keyboard shortcuts for the menu commands.
//!
//! Every binding uses [`egui::Modifiers::COMMAND`], which is Command on macOS
//! and Ctrl elsewhere. Shortcuts never consume a key while a text field has
//! keyboard focus, so typing in the grid editor, the format bar or a dialog
//! always wins.

use egui::{Context, Key, KeyboardShortcut, ModifierNames, Modifiers};

use crate::menu::{MenuAction, MenuTree};

/// The platform's command modifier.
pub const COMMAND: Modifiers = Modifiers::COMMAND;

/// One key binding.
struct Binding {
    action: MenuAction,
    shortcut: KeyboardShortcut,
}

const fn binding(action: MenuAction, key: Key, modifiers: Modifiers) -> Binding {
    Binding {
        action,
        shortcut: KeyboardShortcut::new(modifiers, key),
    }
}

/// Most specific first: Save As before Save and Redo before Undo, because
/// `consume_shortcut` also matches extra Shift and Alt modifiers.
const BINDINGS: &[Binding] = &[
    binding(MenuAction::SaveAs, Key::S, COMMAND.plus(Modifiers::SHIFT)),
    binding(MenuAction::Export, Key::E, COMMAND.plus(Modifiers::SHIFT)),
    binding(MenuAction::Save, Key::S, COMMAND),
    binding(MenuAction::New, Key::N, COMMAND),
    binding(MenuAction::Open, Key::O, COMMAND),
    binding(MenuAction::Close, Key::W, COMMAND),
    binding(MenuAction::Quit, Key::Q, COMMAND),
    binding(MenuAction::Redo, Key::Z, COMMAND.plus(Modifiers::SHIFT)),
    binding(MenuAction::Undo, Key::Z, COMMAND),
    binding(MenuAction::SelectAll, Key::A, COMMAND),
    binding(MenuAction::Cut, Key::X, COMMAND),
    binding(MenuAction::Copy, Key::C, COMMAND),
    binding(MenuAction::Paste, Key::V, COMMAND),
    binding(MenuAction::ToggleFilter, Key::F, COMMAND),
    binding(
        MenuAction::MoveRowUp,
        Key::ArrowUp,
        COMMAND.plus(Modifiers::ALT),
    ),
    binding(
        MenuAction::MoveRowDown,
        Key::ArrowDown,
        COMMAND.plus(Modifiers::ALT),
    ),
    binding(
        MenuAction::MoveColumnLeft,
        Key::ArrowLeft,
        COMMAND.plus(Modifiers::ALT),
    ),
    binding(
        MenuAction::MoveColumnRight,
        Key::ArrowRight,
        COMMAND.plus(Modifiers::ALT),
    ),
    binding(MenuAction::DeleteSelection, Key::Backspace, Modifiers::NONE),
    binding(MenuAction::DeleteSelection, Key::Delete, Modifiers::NONE),
];

/// The shortcut bound to `action`, if any.
pub fn shortcut(action: MenuAction) -> Option<KeyboardShortcut> {
    BINDINGS
        .iter()
        .find(|binding| binding.action == action)
        .map(|binding| binding.shortcut)
}

/// The localized hint shown beside the menu item.
pub fn hint(action: MenuAction) -> Option<String> {
    let shortcut = shortcut(action)?;
    let is_mac = cfg!(target_os = "macos");
    let names = if is_mac {
        ModifierNames::SYMBOLS
    } else {
        ModifierNames::NAMES
    };
    Some(shortcut.format(&names, is_mac))
}

/// Consumes one shortcut whose menu item is enabled.
///
/// This is the dispatch entry point: it applies the same capability checks as
/// the menu, so ⌘S cannot save a document whose Save item is disabled.
pub fn consume_enabled(ctx: &Context, tree: &MenuTree) -> Option<MenuAction> {
    let action = consume(ctx)?;
    crate::menu::is_enabled(tree, action).then_some(action)
}

/// Consumes one command shortcut from this frame's input.
///
/// Returns `None` while a text field has focus; the shortcut is left in the
/// event queue so the field receives the key.
pub fn consume(ctx: &Context) -> Option<MenuAction> {
    if ctx.text_edit_focused() {
        return None;
    }
    let mut chosen = None;
    ctx.input_mut(|input| {
        for binding in BINDINGS {
            if input.consume_shortcut(&binding.shortcut) {
                chosen = Some(binding.action);
                break;
            }
        }
    });
    chosen
}
