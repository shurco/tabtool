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
//! Every visible widget string is supplied from `tabtool-i18n`; widgets that
//! draw their own labels, such as the grid's context menus, take a
//! [`tabtool_i18n::Language`] in their options. This crate contains no
//! interface copy of its own.
//!
//! The `UiHarness` is behind the off-by-default `test-support` feature, so
//! normal consumers do not compile `egui_kittest`, wgpu or the image diffing
//! dependencies. This crate's own tests enable the feature through a self
//! dev-dependency.

pub mod about;
pub mod document;
pub mod export;
pub mod filter;
pub mod fonts;
mod format_options;
pub mod grid;
#[cfg(feature = "test-support")]
pub mod harness;
pub mod settings;
pub mod theme;
pub mod widgets;

pub use fonts::install_fonts;
#[cfg(feature = "test-support")]
pub use harness::UiHarness;
pub use theme::{Colors, Metrics, Theme, ThemeMode, Typography, apply_theme, apply_themes};
