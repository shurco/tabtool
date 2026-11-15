//! JNI plumbing: the cached bridge class and the typed static-method calls.

use std::sync::OnceLock;

use jni::objects::{GlobalRef, JIntArray, JObject, JObjectArray, JString, JValue, JValueOwned};
use jni::sys::jobject;
use jni::{JNIEnv, JavaVM};
use log::{error, info};

const BRIDGE_CLASS: &str = "app/shurco/tabtool/TabToolBridge";

pub(super) struct Bridge {
    vm: JavaVM,
    pub(super) class: GlobalRef,
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
pub(super) fn with_env<T>(
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
pub(super) fn log_jni_error(
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

pub(super) fn call_string(
    method: &str,
    signature: &str,
    args: &[JValue<'_, '_>],
) -> Option<String> {
    with_env(method, None, |env, bridge| {
        let value = env.call_static_method(&bridge.class, method, signature, args)?;
        take_string(env, value)
    })
}

pub(super) fn call_string_1(method: &str, first: &str, signature: &str) -> Option<String> {
    with_env(method, None, |env, bridge| {
        let first = env.new_string(first)?;
        let value =
            env.call_static_method(&bridge.class, method, signature, &[JValue::from(&first)])?;
        take_string(env, value)
    })
}

pub(super) fn call_string_2(
    method: &str,
    first: &str,
    second: &str,
    signature: &str,
) -> Option<String> {
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

pub(super) fn call_bool(method: &str) -> bool {
    with_env(method, false, |env, bridge| {
        match env.call_static_method(&bridge.class, method, "()Z", &[])? {
            JValueOwned::Bool(value) => Ok(value != 0),
            _ => Err(jni::errors::Error::WrongJValueType("boolean", "other")),
        }
    })
}

pub(super) fn call_int_array(method: &str) -> Option<Vec<i32>> {
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

pub(super) fn call_string_array(method: &str) -> Option<Vec<String>> {
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

pub(super) fn call_void(method: &str, signature: &str, args: &[JValue<'_, '_>]) {
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
