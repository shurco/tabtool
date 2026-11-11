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
    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.ok()?;
    let (activity_context, class) = {
        let mut env = vm.attach_current_thread().ok()?;
        let context = unsafe { JObject::from_raw(context.context() as jobject) };
        let context = env.new_global_ref(context).ok()?;

        let class = bridge_class(&mut env, context.as_obj())?;
        (context, env.new_global_ref(class).ok()?)
    };

    info!("JNI bridge initialized");
    Some(Bridge {
        vm,
        context: activity_context,
        class,
    })
}

fn bridge_class<'local>(
    env: &mut JNIEnv<'local>,
    context: &JObject<'local>,
) -> Option<JObject<'local>> {
    // `FindClass` on the native thread only sees the boot class path, so load the
    // bridge through the application's class loader instead.
    let name = env.new_string(BRIDGE_CLASS.replace('/', ".")).ok()?;
    let loader = env
        .call_method(context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .ok()?
        .l()
        .ok()?;
    if loader.is_null() {
        error!("Android context has no class loader");
        return None;
    }

    match env.call_method(
        &loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::from(&name)],
    ) {
        Ok(class) => class.l().ok(),
        Err(error) => {
            log_jni_error(env, "loadClass", &error);
            None
        }
    }
}

fn call_static_string(method: &str, signature: &str) -> Option<String> {
    let bridge = bridge()?;
    let mut env = bridge.vm.attach_current_thread().ok()?;

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

fn log_jni_error(env: &mut JNIEnv<'_>, method: &str, error: &jni::errors::Error) -> bool {
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
    let Ok(mut env) = bridge.vm.attach_current_thread() else {
        return;
    };
    let Ok(text) = env.new_string(text) else {
        return;
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

/// Reads the current window insets in dp.
pub fn insets() -> Insets {
    let Some(bridge) = bridge() else {
        return Insets::default();
    };
    let Ok(mut env) = bridge.vm.attach_current_thread() else {
        return Insets::default();
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
    let Ok(length) = env.get_array_length(&array) else {
        return Insets::default();
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
