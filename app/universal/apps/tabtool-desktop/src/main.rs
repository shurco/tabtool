//! The TabTool desktop entry point.

use eframe::egui;
use tabtool_desktop::app::DesktopApp;
use tabtool_platform::{DesktopPlatform, Settings, settings_path};

fn main() -> eframe::Result {
    if smoke_requested(std::env::args().skip(1)) {
        println!("tabtool {}", tabtool_core::VERSION);
        return Ok(());
    }

    let settings = settings_path()
        .map(|path| Settings::load(&path))
        .unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TabTool")
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TabTool",
        options,
        Box::new(move |creation| {
            tabtool_ui::install_fonts(&creation.egui_ctx);
            tabtool_ui::apply_themes(&creation.egui_ctx);
            let mut app = DesktopApp::new(settings, DesktopPlatform);
            app.new_document();
            Ok(Box::new(app))
        }),
    )
}

fn smoke_requested(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|arg| arg == "--smoke")
}

#[cfg(test)]
mod tests {
    use super::smoke_requested;

    #[test]
    fn detects_smoke_flag_among_other_arguments() {
        let args = ["ignored".to_owned(), "--smoke".to_owned()];

        assert!(smoke_requested(args.into_iter()));
    }

    #[test]
    fn ignores_arguments_without_smoke_flag() {
        let args = ["--windowed".to_owned()];

        assert!(!smoke_requested(args.into_iter()));
    }
}
