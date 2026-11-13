#![forbid(unsafe_code)]

//! Design tokens, widgets and the deterministic test harness for the TabTool
//! desktop UI.
//!
//! Start an application with [`install_fonts`], then install the appearance:
//!
//! ```no_run
//! # let ctx = egui::Context::default();
//! tabtool_ui::install_fonts(&ctx);
//! tabtool_ui::apply_themes(&ctx); // both palettes, following the system
//! ```
//!
//! Every visible widget string is supplied by the caller from `tabtool-i18n`;
//! this crate contains no interface copy of its own.

pub mod fonts;
#[cfg(feature = "test-support")]
pub mod harness;
pub mod theme;
pub mod widgets;

pub use fonts::install_fonts;
#[cfg(feature = "test-support")]
pub use harness::UiHarness;
pub use theme::{Colors, Metrics, Theme, ThemeMode, Typography, apply_theme, apply_themes};
