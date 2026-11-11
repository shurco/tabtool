//! JNI bridge to `app.shurco.tabtool.TabToolBridge`.

use std::sync::OnceLock;

use jni::objects::{GlobalRef, JIntArray, JObject, JString, JValue, JValueOwned};
use jni::sys::jobject;
use jni::{JNIEnv, JavaVM};
use log::{error, info};

use crate::insets::Insets;

const BRIDGE_CLASS: &str = "app/shurco/tabtool/TabToolBridge";

struct Bridge {
    vm: JavaVM,
    /// Cached Android context handle from `ndk_context`.
    #[allow(dead_code)]
    context: GlobalRef,
    class: GlobalRef,
}

static BRIDGE: OnceLock<Option<Bridge>> = OnceLock::new();

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

    let (context, class) = {
        let mut env = match vm.attach_current_thread() {
            Ok(env) => env,
            Err(error) => {
                error!("Cannot attach the native thread to the JavaVM: {error}");
                return None;
            }
        };

        let refs = (|| -> jni::errors::Result<(GlobalRef, GlobalRef)> {
            // SAFETY: `ndk_context` holds a live Android context reference. It is
            // copied into a global reference immediately; the borrowed value is not
            // used after this scope.
            let context = unsafe { JObject::from_raw(android_context.context() as jobject) };
            let context = env.new_global_ref(context)?;
            let class = bridge_class(&mut env, context.as_obj())?;
            let class = env.new_global_ref(class)?;
            Ok((context, class))
        })();

        match refs {
            Ok(refs) => refs,
            Err(error) => {
                log_jni_error(&mut env, "init_bridge", &error);
                return None;
            }
        }
    };

    info!("JNI bridge initialized");
    Some(Bridge { vm, context, class })
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

fn call_static_string(method: &str, signature: &str) -> Option<String> {
    let bridge = bridge()?;
    let mut env = match bridge.vm.attach_current_thread() {
        Ok(env) => env,
        Err(error) => {
            error!("Cannot attach the native thread for `{method}`: {error}");
            return None;
        }
    };

    match env.call_static_method(&bridge.class, method, signature, &[]) {
        Ok(JValueOwned::Object(value)) if !value.is_null() => {
            let text = JString::from(value);
            match env.get_string(&text) {
                Ok(text) => Some(text.into()),
                Err(error) => {
                    log_jni_error(&mut env, method, &error);
                    None
                }
            }
        }
        Ok(_) => None,
        Err(error) => {
            log_jni_error(&mut env, method, &error);
            None
        }
    }
}

fn call_static_void(method: &str, signature: &str, args: &[JValue<'_, '_>]) {
    let Some(bridge) = bridge() else {
        return;
    };
    let mut env = match bridge.vm.attach_current_thread() {
        Ok(env) => env,
        Err(error) => {
            error!("Cannot attach the native thread for `{method}`: {error}");
            return;
        }
    };

    if let Err(error) = env.call_static_method(&bridge.class, method, signature, args) {
        log_jni_error(&mut env, method, &error);
    }
}

/// Checks, describes and clears a pending Java exception, then logs the failure.
pub(crate) fn log_jni_error(
    env: &mut JNIEnv<'_>,
    method: &str,
    error: &jni::errors::Error,
) -> bool {
    let pending = env.exception_check().unwrap_or(false);
    if pending {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    error!("JNI call `{method}` failed: {error}");
    pending
}

/// Returns the primary clipboard text, or `None` when it is empty or inaccessible.
pub fn clipboard_get() -> Option<String> {
    call_static_string("clipboardGet", "()Ljava/lang/String;")
}

/// Copies `text` to the system clipboard.
pub fn clipboard_set(text: &str) {
    let Some(bridge) = bridge() else {
        return;
    };
    let mut env = match bridge.vm.attach_current_thread() {
        Ok(env) => env,
        Err(error) => {
            error!("Cannot attach the native thread for `clipboardSet`: {error}");
            return;
        }
    };
    let text = match env.new_string(text) {
        Ok(text) => text,
        Err(error) => {
            log_jni_error(&mut env, "clipboardSet", &error);
            return;
        }
    };
    let args = [JValue::from(&text)];

    if let Err(error) = env.call_static_method(
        &bridge.class,
        "clipboardSet",
        "(Ljava/lang/String;)V",
        &args,
    ) {
        log_jni_error(&mut env, "clipboardSet", &error);
    }
}

/// Opens the document picker and returns the URI picked by a previous call, if any.
pub fn open_document() -> Option<String> {
    call_static_string("openDocument", "()Ljava/lang/String;")
}

/// Opens the native `EditText` overlay at a physical-pixel rectangle.
pub fn show_text_editor(left: i32, top: i32, width: i32, height: i32, text: &str, multiline: bool) {
    let Some(bridge) = bridge() else {
        return;
    };
    let mut env = match bridge.vm.attach_current_thread() {
        Ok(env) => env,
        Err(error) => {
            error!("Cannot attach the native thread for `showTextEditor`: {error}");
            return;
        }
    };
    let text = match env.new_string(text) {
        Ok(text) => text,
        Err(error) => {
            log_jni_error(&mut env, "showTextEditor", &error);
            return;
        }
    };
    let args = [
        JValue::Int(left),
        JValue::Int(top),
        JValue::Int(width),
        JValue::Int(height),
        JValue::from(&text),
        JValue::Bool(u8::from(multiline)),
    ];

    if let Err(error) = env.call_static_method(
        &bridge.class,
        "showTextEditor",
        "(IIIILjava/lang/String;Z)V",
        &args,
    ) {
        log_jni_error(&mut env, "showTextEditor", &error);
    }
}

/// Removes the native `EditText` overlay without reporting an outcome.
pub fn hide_text_editor() {
    call_static_void("hideTextEditor", "()V", &[]);
}

/// Reads the current window insets in dp.
pub fn insets() -> Insets {
    let Some(bridge) = bridge() else {
        return Insets::default();
    };
    let mut env = match bridge.vm.attach_current_thread() {
        Ok(env) => env,
        Err(error) => {
            error!("Cannot attach the native thread for `insets`: {error}");
            return Insets::default();
        }
    };

    let value = match env.call_static_method(&bridge.class, "insets", "()[I", &[]) {
        Ok(JValueOwned::Object(value)) if !value.is_null() => value,
        Ok(_) => return Insets::default(),
        Err(error) => {
            log_jni_error(&mut env, "insets", &error);
            return Insets::default();
        }
    };

    let array = JIntArray::from(value);
    let length = match env.get_array_length(&array) {
        Ok(length) => length,
        Err(error) => {
            log_jni_error(&mut env, "insets", &error);
            return Insets::default();
        }
    };
    let mut buffer = vec![0i32; length.max(0) as usize];
    if let Err(error) = env.get_int_array_region(&array, 0, &mut buffer) {
        log_jni_error(&mut env, "insets", &error);
        return Insets::default();
    }

    Insets::from_array(&buffer).unwrap_or_default()
}

/// Calls a bridge method that throws a `SecurityException` and reports whether
/// the JNI exception was contained (cleared) instead of aborting the app.
pub fn throw_test_security_exception() -> bool {
    let Some(bridge) = bridge() else {
        return false;
    };
    let Ok(mut env) = bridge.vm.attach_current_thread() else {
        return false;
    };

    match env.call_static_method(&bridge.class, "throwSecurityException", "()V", &[]) {
        Ok(_) => false,
        Err(error) => log_jni_error(&mut env, "throwSecurityException", &error),
    }
}

pub fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("TabTool"),
    );
}
