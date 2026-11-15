//! TabTool's Android application.
//!
//! The single-window app composes the shared mobile views: the documents
//! library, one editor, sheets, autosave, intents and process-death state.
//! Configuration lives in [`app`], [`nav`], [`intents`] and [`lifecycle`];
//! the Android-only JNI adapters live in [`android_host`].

pub mod app;
pub mod intents;
pub mod library;
pub mod lifecycle;
pub mod nav;

#[cfg(target_os = "android")]
pub mod android_host;

#[cfg(target_os = "android")]
static EGUI_CONTEXT: std::sync::OnceLock<eframe::egui::Context> = std::sync::OnceLock::new();

/// Wakes the egui pass from a JNI callback.
///
/// Native editor commits, finished intent copies and the activity going to the
/// background arrive on the Java UI thread. Without a repaint request the app
/// would only notice them on the next touch, so the callback wakes egui.
///
/// # Safety
///
/// The JVM calls this on the UI thread; the `JNIEnv` and object references are
/// valid for the duration of the call and are not used.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolBridge_nativeWake(
    _env: jni::JNIEnv<'_>,
    _this: jni::objects::JObject<'_>,
) {
    if let Some(context) = EGUI_CONTEXT.get() {
        context.request_repaint();
    }
}

/// Flushes the pending edit before the activity pauses.
///
/// The Java UI thread blocks for up to 400 ms while the native loop runs one
/// pass and acknowledges. A timed-out save is still retried by the next frame,
/// so the activity never loses data silently.
///
/// # Safety
///
/// The JVM calls this on the UI thread; both references are valid for the call
/// and are not used.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolBridge_nativeSaveOnBackground(
    _env: jni::JNIEnv<'_>,
    _this: jni::objects::JObject<'_>,
) -> jni::sys::jboolean {
    use std::time::{Duration, Instant};

    const BACKGROUND_TIMEOUT: Duration = Duration::from_millis(400);

    android_host::begin_background_save();
    if let Some(context) = EGUI_CONTEXT.get() {
        context.request_repaint();
    }
    let deadline = Instant::now() + BACKGROUND_TIMEOUT;
    while !android_host::background_save_acknowledged() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    if android_host::background_save_acknowledged() {
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(activity: winit::platform::android::activity::AndroidApp) {
    use eframe::egui;

    init_logging();
    log::info!("TabTool {} starting", tabtool_core::VERSION);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("TabTool"),
        android_app: Some(activity),
        wgpu_options: gl_only_wgpu(),
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "TabTool",
        options,
        Box::new(|creation| {
            if let Err(error) = tabtool_ui::install_fonts(&creation.egui_ctx) {
                log::error!("system fonts are unavailable: {error:?}");
            }
            tabtool_ui::apply_themes(&creation.egui_ctx);
            let _ = EGUI_CONTEXT.set(creation.egui_ctx.clone());
            Ok(Box::new(android_host::create_app()))
        }),
    ) {
        log::error!("eframe failed to run: {error}");
    }
}

#[cfg(target_os = "android")]
impl eframe::App for app::AndroidApp {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.poll(std::time::Instant::now());
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
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
