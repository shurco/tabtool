//! TabTool Android host and the platform-services smoke UI.
//!
//! The spike exercises the production platform layer: the native text editor,
//! SAF import/export, clipboard, share, insets, system languages, links and
//! back. It is replaced by the real mobile app in Plan 5 Task 6.

#[cfg(target_os = "android")]
mod spike_app;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::egui;

    init_logging();
    log::info!("TabTool {} starting", tabtool_core::VERSION);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("TabTool"),
        android_app: Some(app),
        wgpu_options: gl_only_wgpu(),
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "TabTool",
        options,
        Box::new(|_cc| Ok(Box::new(spike_app::SpikeApp::default()))),
    ) {
        log::error!("eframe failed to run: {error}");
    }
}

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("TabTool"),
    );
}

/// The Android emulator's Vulkan driver (`vulkan.ranchu`) segfaults while wgpu
/// names objects during adapter setup. GLES works, so pin wgpu to its GL backend.
#[cfg(target_os = "android")]
fn gl_only_wgpu() -> eframe::egui_wgpu::WgpuConfiguration {
    let mut configuration = eframe::egui_wgpu::WgpuConfiguration::default();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut configuration.wgpu_setup {
        setup.instance_descriptor.backends = eframe::wgpu::Backends::GL;
    }
    configuration
}
