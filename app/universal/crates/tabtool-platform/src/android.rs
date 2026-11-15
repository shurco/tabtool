//! Android platform services through the Kotlin `TabToolBridge`.
//!
//! The JavaVM, the bridge class and the documents directory are resolved once
//! and cached. Every call checks, describes and clears a pending Java
//! exception before it is logged, so a failed bridge method can never abort the
//! native app.
//!
//! The bridge encodes its typed results as strings: `OK`, `OK:<value>`,
//! `CANCELLED` or `ERROR:<message>`. Pending file operations return `null` and
//! are polled again later.

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use jni::objects::{GlobalRef, JIntArray, JObject, JObjectArray, JString, JValue, JValueOwned};
use jni::sys::jobject;
use jni::{JNIEnv, JavaVM};
use log::{error, info};

use crate::bridge_result::{BridgeResult, parse_result};
use crate::locale::{SystemInfo, decimal_separator, normalize_tag};
use crate::services::{
    BackHandler, Clipboard, CopyOutcome, DocumentFiles, ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE,
    Insets, OpenUrl, PlatformError, PlatformErrors, Share,
};
use crate::text_input::{EditorSessions, TextInput, TextInputOutcome, TextInputRequest};

const BRIDGE_CLASS: &str = "app/shurco/tabtool/TabToolBridge";

struct Bridge {
    vm: JavaVM,
    class: GlobalRef,
}

static BRIDGE: OnceLock<Option<Bridge>> = OnceLock::new();
static DOCUMENTS_DIR: OnceLock<PathBuf> = OnceLock::new();
static SESSIONS: EditorSessions = EditorSessions::new();
static ERRORS: Mutex<VecDeque<PlatformError>> = Mutex::new(VecDeque::new());
static EXPORT_OUTCOMES: Mutex<VecDeque<CopyOutcome>> = Mutex::new(VecDeque::new());

/// A document another app asked TabTool to open.
///
/// `Staged` names the copied file inside the app's private documents
/// directory and the original display name. The app picks a collision-free
/// final name before it opens the copy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentImport {
    /// The content was copied and waits to be adopted.
    Staged {
        /// The file name inside the documents directory.
        file_name: String,
        /// The display name reported by the sending app.
        display_name: String,
    },
    /// The copy failed; the string is the diagnostic detail.
    Failed(String),
}

/// The Android platform services.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidPlatform;

impl AndroidPlatform {
    /// Whether a SAF import or export is still running.
    ///
    /// The neutral [`DocumentFiles`] methods return their result on a later
    /// call. The app polls this first so it does not relaunch the picker after
    /// the previous request.
    #[must_use]
    pub fn document_operation_pending(self) -> bool {
        call_bool("isDocumentOperationPending")
    }

    /// Closes the activity when the app's navigation stack cannot take back.
    pub fn finish_activity(self) {
        call_void("finishActivity", "()V", &[]);
    }

    /// Whether an import picker started earlier is still open.
    #[must_use]
    pub fn import_pending(self) -> bool {
        call_bool("isImportPending")
    }

    /// Whether an export picker or copy started earlier is still running.
    #[must_use]
    pub fn export_pending(self) -> bool {
        call_bool("isExportPending")
    }

    /// Takes the outcome of the last completed export.
    ///
    /// [`DocumentFiles::export_copy`] answers with `bool` when the operation
    /// finishes; this keeps the typed result so an async cancel is not shown
    /// as a failure. Pending calls push nothing.
    #[must_use]
    pub fn take_export_outcome(self) -> Option<CopyOutcome> {
        EXPORT_OUTCOMES
            .lock()
            .map(|mut outcomes| outcomes.pop_front())
            .unwrap_or_else(|poisoned| poisoned.into_inner().pop_front())
    }

    /// The app's data directory: settings and process-death state live here.
    #[must_use]
    pub fn files_dir(self) -> Option<PathBuf> {
        call_string("filesDir", "()Ljava/lang/String;", &[]).map(PathBuf::from)
    }

    /// The device's UTC offset in minutes and whether it uses a 12-hour clock.
    #[must_use]
    pub fn clock(self) -> Option<(i32, bool)> {
        let values = call_int_array("clock")?;
        let offset = *values.first()?;
        let twelve_hour = values.get(1).is_some_and(|value| *value != 0);
        Some((offset, twelve_hour))
    }

    /// Takes one transition to the background.
    #[must_use]
    pub fn take_background_event(self) -> bool {
        call_bool("takeBackgroundEvent")
    }

    /// Takes one document another app asked TabTool to open.
    ///
    /// The first call after the copy completes reports the staged file; later
    /// calls report `None` until another intent arrives.
    #[must_use]
    pub fn take_intent_import(self) -> Option<IntentImport> {
        let raw = call_string("takeIntentImport", "()Ljava/lang/String;", &[])?;
        match parse_result(&raw) {
            BridgeResult::Ok(Some(value)) => match value.split_once('\n') {
                Some((file_name, display_name)) if !file_name.is_empty() => {
                    Some(IntentImport::Staged {
                        file_name: file_name.to_owned(),
                        display_name: display_name.to_owned(),
                    })
                }
                _ => Some(IntentImport::Failed(format!(
                    "the intent result is malformed: {value}"
                ))),
            },
            BridgeResult::Ok(None) | BridgeResult::Cancelled => None,
            BridgeResult::Failed(message) => Some(IntentImport::Failed(message)),
        }
    }
}

impl TextInput for AndroidPlatform {
    fn open(&self, request: TextInputRequest) {
        SESSIONS.reset();
        if !request.is_valid() {
            error!(
                "Refusing an editor request with rect {:?}: the size is not usable",
                request.rect
            );
            return;
        }

        let [left, top, width, height] = request.rect;
        let shown = with_env("showTextEditor", false, |env, bridge| {
            let text = env.new_string(&request.text)?;
            let args = [
                JValue::Float(left),
                JValue::Float(top),
                JValue::Float(width),
                JValue::Float(height),
                JValue::from(&text),
                JValue::Bool(u8::from(request.multiline)),
            ];
            match env.call_static_method(
                &bridge.class,
                "showTextEditor",
                "(FFFFLjava/lang/String;Z)Z",
                &args,
            )? {
                JValueOwned::Bool(value) => Ok(value != 0),
                _ => Err(jni::errors::Error::WrongJValueType("boolean", "other")),
            }
        });
        if shown {
            SESSIONS.activate();
        } else {
            error!("The native editor did not open; no session is active");
        }
    }

    fn close(&self) {
        SESSIONS.reset();
        call_void("hideTextEditor", "()V", &[]);
    }

    fn commit(&self) {
        call_void("commitTextEditor", "()V", &[]);
    }

    fn move_to(&self, rect: [f32; 4]) {
        let [left, top, width, height] = rect;
        if !rect.iter().all(|value| value.is_finite()) || width <= 0.0 || height <= 0.0 {
            return;
        }
        call_void(
            "moveTextEditor",
            "(FFFF)V",
            &[
                JValue::Float(left),
                JValue::Float(top),
                JValue::Float(width),
                JValue::Float(height),
            ],
        );
    }

    fn poll(&self) -> Option<TextInputOutcome> {
        SESSIONS.poll()
    }

    fn is_active(&self) -> bool {
        SESSIONS.is_active()
    }
}

impl DocumentFiles for AndroidPlatform {
    fn documents_dir(&self) -> PathBuf {
        crate::services::cached_path(&DOCUMENTS_DIR, || {
            call_string("documentsDir", "()Ljava/lang/String;", &[]).map(PathBuf::from)
        })
        .unwrap_or_default()
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        crate::services::table_files(&self.documents_dir())
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        let raw = call_string("importDocument", "()Ljava/lang/String;", &[])?;
        match parse_result(&raw) {
            BridgeResult::Ok(Some(name)) => {
                let path = self.documents_dir().join(name);
                if path.is_file() {
                    Some(path)
                } else {
                    record_error(
                        ERROR_OPEN,
                        format!("the imported document is missing at {}", path.display()),
                    );
                    None
                }
            }
            BridgeResult::Ok(None) => None,
            BridgeResult::Cancelled => None,
            BridgeResult::Failed(message) => {
                record_error(ERROR_OPEN, format!("import: {message}"));
                None
            }
        }
    }

    fn export_copy(&self, source: &Path, suggested_name: &str) -> bool {
        let source = source.to_string_lossy();
        let Some(raw) = call_string_2(
            "exportDocument",
            &source,
            suggested_name,
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        ) else {
            return false;
        };
        match parse_result(&raw) {
            BridgeResult::Ok(_) => {
                record_export_outcome(CopyOutcome::Copied);
                true
            }
            BridgeResult::Cancelled => {
                record_export_outcome(CopyOutcome::Cancelled);
                false
            }
            BridgeResult::Failed(message) => {
                record_export_outcome(CopyOutcome::Failed);
                record_error(ERROR_SAVE, format!("export: {message}"));
                false
            }
        }
    }

    fn delete(&self, path: &Path) -> io::Result<()> {
        crate::services::ensure_within(&self.documents_dir(), path)?;
        std::fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let directory = self.documents_dir();
        crate::services::ensure_within(&directory, from)?;
        crate::services::ensure_within(&directory, to)?;
        std::fs::rename(from, to)
    }
}

impl Insets for AndroidPlatform {
    fn safe_area(&self) -> [f32; 4] {
        match call_int_array("insets") {
            Some(values) if values.len() >= 5 => [
                values[0] as f32,
                values[1] as f32,
                values[2] as f32,
                values[3] as f32,
            ],
            _ => [0.0; 4],
        }
    }

    fn keyboard_height(&self) -> f32 {
        call_int_array("insets")
            .and_then(|values| values.get(4).copied())
            .map_or(0.0, |height| height as f32)
    }
}

impl Share for AndroidPlatform {
    fn share(&self, path: &Path) {
        let path = path.to_string_lossy();
        let Some(raw) = call_string_1("shareFile", &path, "(Ljava/lang/String;)Ljava/lang/String;")
        else {
            return;
        };
        if let BridgeResult::Failed(message) = parse_result(&raw) {
            record_error(ERROR_GENERIC, format!("share: {message}"));
        }
    }
}

impl BackHandler for AndroidPlatform {
    fn take_back(&self) -> bool {
        call_bool("takeBack")
    }
}

impl Clipboard for AndroidPlatform {
    fn get(&self) -> Option<String> {
        call_string("clipboardGet", "()Ljava/lang/String;", &[])
    }

    fn set(&self, text: &str) {
        with_env("clipboardSet", (), |env, bridge| {
            let text = env.new_string(text)?;
            env.call_static_method(
                &bridge.class,
                "clipboardSet",
                "(Ljava/lang/String;)V",
                &[JValue::from(&text)],
            )?;
            Ok(())
        });
    }
}

impl OpenUrl for AndroidPlatform {
    fn open(&self, url: &str) {
        let Some(raw) = call_string_1("openUrl", url, "(Ljava/lang/String;)Ljava/lang/String;")
        else {
            return;
        };
        if let BridgeResult::Failed(message) = parse_result(&raw) {
            record_error(ERROR_GENERIC, format!("open {url}: {message}"));
        }
    }
}

impl SystemInfo for AndroidPlatform {
    fn languages(&self) -> Vec<String> {
        call_string_array("languages").unwrap_or_default()
    }

    fn locale(&self) -> String {
        self.languages()
            .first()
            .map_or_else(|| "en_US".to_owned(), |tag| normalize_tag(tag))
    }

    fn decimal_separator(&self) -> String {
        decimal_separator(&self.locale())
    }
}

impl PlatformErrors for AndroidPlatform {
    fn take_error(&self) -> Option<PlatformError> {
        ERRORS
            .lock()
            .map(|mut errors| errors.pop_front())
            .unwrap_or_else(|poisoned| poisoned.into_inner().pop_front())
    }
}

/// Queues a failure for the UI and logs its diagnostic detail.
fn record_error(message_key: &'static str, detail: String) {
    error!("Platform error ({message_key}): {detail}");
    let mut errors = match ERRORS.lock() {
        Ok(errors) => errors,
        Err(poisoned) => poisoned.into_inner(),
    };
    if errors.len() >= 8 {
        let _ = errors.pop_front();
    }
    errors.push_back(PlatformError::new(message_key, detail));
}

/// Remembers the typed outcome of a finished export.
fn record_export_outcome(outcome: CopyOutcome) {
    let mut outcomes = match EXPORT_OUTCOMES.lock() {
        Ok(outcomes) => outcomes,
        Err(poisoned) => poisoned.into_inner(),
    };
    if outcomes.len() >= 4 {
        let _ = outcomes.pop_front();
    }
    outcomes.push_back(outcome);
}

// --- JNI plumbing -----------------------------------------------------------

fn bridge() -> Option<&'static Bridge> {
    BRIDGE
        .get_or_init(|| {
            // SAFETY: android-activity initializes `ndk_context` before `android_main`.
            unsafe { init_bridge() }
        })
        .as_ref()
}

unsafe fn init_bridge() -> Option<Bridge> {
    let android_context = ndk_context::android_context();
    // SAFETY: `ndk_context` was initialized by android-activity with the process
    // `JavaVM` before `android_main`; the pointer is valid for the process lifetime.
    let vm = match unsafe { JavaVM::from_raw(android_context.vm().cast()) } {
        Ok(vm) => vm,
        Err(error) => {
            error!("Cannot wrap the Android JavaVM: {error}");
            return None;
        }
    };

    let class = {
        let mut env = match vm.attach_current_thread() {
            Ok(env) => env,
            Err(error) => {
                error!("Cannot attach the native thread to the JavaVM: {error}");
                return None;
            }
        };

        let class = (|| -> jni::errors::Result<GlobalRef> {
            // SAFETY: `ndk_context` holds a live Android context reference for
            // the process lifetime. It is borrowed only to reach the app's
            // class loader; the borrowed value is not used after this scope.
            let context = unsafe { JObject::from_raw(android_context.context() as jobject) };
            let class = bridge_class(&mut env, &context)?;
            env.new_global_ref(class)
        })();

        match class {
            Ok(class) => class,
            Err(error) => {
                log_jni_error(&mut env, "init_bridge", &error);
                return None;
            }
        }
    };

    info!("JNI bridge initialized");
    Some(Bridge { vm, class })
}

fn bridge_class<'local>(
    env: &mut JNIEnv<'local>,
    context: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    // `FindClass` on the native thread only sees the boot class path, so load the
    // bridge through the application's class loader instead.
    let name = env.new_string(BRIDGE_CLASS.replace('/', "."))?;
    let loader = env
        .call_method(context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    if loader.is_null() {
        return Err(jni::errors::Error::NullPtr("bridge class loader"));
    }

    env.call_method(
        &loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::from(&name)],
    )?
    .l()
}

/// Attaches the calling thread and runs `call`, containing every JNI failure.
fn with_env<T>(
    method: &str,
    fallback: T,
    call: impl FnOnce(&mut JNIEnv<'_>, &'static Bridge) -> jni::errors::Result<T>,
) -> T {
    let Some(bridge) = bridge() else {
        error!("The JNI bridge is unavailable for `{method}`");
        return fallback;
    };
    let mut env = match bridge.vm.attach_current_thread() {
        Ok(env) => env,
        Err(error) => {
            error!("Cannot attach the native thread for `{method}`: {error}");
            return fallback;
        }
    };

    match call(&mut env, bridge) {
        Ok(value) => value,
        Err(error) => {
            log_jni_error(&mut env, method, &error);
            fallback
        }
    }
}

/// Checks, describes and clears a pending Java exception, then logs the failure.
fn log_jni_error(env: &mut JNIEnv<'_>, method: &str, error: &jni::errors::Error) -> bool {
    let pending = env.exception_check().unwrap_or(false);
    if pending {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    error!("JNI call `{method}` failed: {error}");
    pending
}

fn call_string(method: &str, signature: &str, args: &[JValue<'_, '_>]) -> Option<String> {
    with_env(method, None, |env, bridge| {
        let value = env.call_static_method(&bridge.class, method, signature, args)?;
        take_string(env, value)
    })
}

fn call_string_1(method: &str, first: &str, signature: &str) -> Option<String> {
    with_env(method, None, |env, bridge| {
        let first = env.new_string(first)?;
        let value =
            env.call_static_method(&bridge.class, method, signature, &[JValue::from(&first)])?;
        take_string(env, value)
    })
}

fn call_string_2(method: &str, first: &str, second: &str, signature: &str) -> Option<String> {
    with_env(method, None, |env, bridge| {
        let first = env.new_string(first)?;
        let second = env.new_string(second)?;
        let value = env.call_static_method(
            &bridge.class,
            method,
            signature,
            &[JValue::from(&first), JValue::from(&second)],
        )?;
        take_string(env, value)
    })
}

fn call_bool(method: &str) -> bool {
    with_env(method, false, |env, bridge| {
        match env.call_static_method(&bridge.class, method, "()Z", &[])? {
            JValueOwned::Bool(value) => Ok(value != 0),
            _ => Err(jni::errors::Error::WrongJValueType("boolean", "other")),
        }
    })
}

fn call_int_array(method: &str) -> Option<Vec<i32>> {
    with_env(method, None, |env, bridge| {
        let value = env.call_static_method(&bridge.class, method, "()[I", &[])?;
        let JValueOwned::Object(object) = value else {
            return Err(jni::errors::Error::WrongJValueType("int[]", "other"));
        };
        if object.is_null() {
            return Ok(None);
        }
        let array = JIntArray::from(object);
        let length = env.get_array_length(&array)?;
        let mut values = vec![0i32; length.max(0) as usize];
        if !values.is_empty() {
            env.get_int_array_region(&array, 0, &mut values)?;
        }
        Ok(Some(values))
    })
}

fn call_string_array(method: &str) -> Option<Vec<String>> {
    with_env(method, None, |env, bridge| {
        let value = env.call_static_method(&bridge.class, method, "()[Ljava/lang/String;", &[])?;
        let JValueOwned::Object(object) = value else {
            return Err(jni::errors::Error::WrongJValueType("String[]", "other"));
        };
        if object.is_null() {
            return Ok(None);
        }
        let array = JObjectArray::from(object);
        let length = env.get_array_length(&array)?;
        let mut strings = Vec::with_capacity(length.max(0) as usize);
        for index in 0..length {
            let element = env.get_object_array_element(&array, index)?;
            if element.is_null() {
                continue;
            }
            let text = JString::from(element);
            strings.push(env.get_string(&text)?.into());
        }
        Ok(Some(strings))
    })
}

fn call_void(method: &str, signature: &str, args: &[JValue<'_, '_>]) {
    with_env(method, (), |env, bridge| {
        env.call_static_method(&bridge.class, method, signature, args)?;
        Ok(())
    });
}

fn take_string(
    env: &mut JNIEnv<'_>,
    value: JValueOwned<'_>,
) -> jni::errors::Result<Option<String>> {
    let JValueOwned::Object(object) = value else {
        return Ok(None);
    };
    if object.is_null() {
        return Ok(None);
    }
    let text = JString::from(object);
    Ok(Some(env.get_string(&text)?.into()))
}

// --- JNI callbacks ----------------------------------------------------------

/// JNI callback from `TabToolBridge.nativeTextCommitted`.
///
/// # Safety
///
/// The JVM calls this on the UI thread of the attached app; the `JNIEnv` and
/// `JString` arguments are valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolBridge_nativeTextCommitted(
    mut env: JNIEnv<'_>,
    _this: JObject<'_>,
    text: JString<'_>,
) {
    match env.get_string(&text) {
        Ok(text) => SESSIONS.finish(TextInputOutcome::Committed(text.into())),
        Err(error) => {
            log_jni_error(&mut env, "nativeTextCommitted", &error);
            error!("Dropping a text commit that could not be read from the JVM");
        }
    }
}

/// JNI callback from `TabToolBridge.nativeTextCancelled`.
///
/// # Safety
///
/// The JVM calls this on the UI thread of the attached app; both references are
/// valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolBridge_nativeTextCancelled(
    _env: JNIEnv<'_>,
    _this: JObject<'_>,
) {
    SESSIONS.finish(TextInputOutcome::Cancelled);
}
