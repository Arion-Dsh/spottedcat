use crate::scenes::SceneFactory;
#[cfg(target_os = "android")]
use android_activity::AndroidApp;
#[cfg(target_os = "android")]
use std::sync::Mutex;
#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();
#[cfg(target_os = "android")]
static JVM: OnceLock<jni::JavaVM> = OnceLock::new();
static ACTIVITY: OnceLock<jni::objects::GlobalRef> = OnceLock::new();
static FLOATING_SERVICE_CLASS: OnceLock<String> = OnceLock::new();
static FLOATING_SURFACE: Mutex<Option<jni::objects::GlobalRef>> = Mutex::new(None);
static FLOATING_SCENE_FACTORY: OnceLock<SceneFactory> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn get_jvm() -> Option<&'static jni::JavaVM> {
    JVM.get()
}

#[cfg(target_os = "android")]
pub fn get_activity() -> Option<&'static jni::objects::GlobalRef> {
    ACTIVITY.get()
}

#[cfg(target_os = "android")]
pub fn get_app() -> Option<AndroidApp> {
    ANDROID_APP.get().cloned()
}

#[cfg(target_os = "android")]
fn find_class<'a>(
    env: &mut jni::JNIEnv<'a>,
    class_name: &str,
) -> anyhow::Result<jni::objects::JClass<'a>> {
    // If it starts with a known system class, using find_class is fine
    if class_name.starts_with("android/") || class_name.starts_with("java/") {
        return Ok(env.find_class(class_name)?);
    }

    // For app classes, we must use the app's ClassLoader because we might be on a background thread
    let activity = ACTIVITY
        .get()
        .ok_or_else(|| anyhow::anyhow!("ACTIVITY not initialized"))?
        .as_obj();
    let class_loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;

    let class_name_java = env.new_string(class_name.replace('/', "."))?;
    let class_obj = env
        .call_method(
            class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[(&class_name_java).into()],
        )?
        .l()?;

    Ok(class_obj.into())
}

#[cfg(target_os = "android")]
pub fn init(app: AndroidApp) {
    let _ = ANDROID_APP.set(app.clone());

    unsafe {
        let Ok(vm) = jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _) else {
            eprintln!("[spot][android] failed to create JavaVM from raw pointer");
            return;
        };
        let activity = jni::objects::JObject::from_raw(app.activity_as_ptr() as *mut _);
        let _ = JVM.set(vm);
        let Some(jvm) = JVM.get() else {
            eprintln!("[spot][android] JVM was not stored during init");
            return;
        };
        let Ok(mut env) = jvm.attach_current_thread() else {
            eprintln!("[spot][android] failed to attach current thread during init");
            return;
        };
        let Ok(activity_ref) = env.new_global_ref(activity) else {
            eprintln!("[spot][android] failed to create global activity ref");
            return;
        };
        let _ = ACTIVITY.set(activity_ref);

        // If service class was already set, register it now
        if let Some(class_name) = floating_window_service_class() {
            if let Err(err) = register_floating_window_methods(&mut env, class_name) {
                eprintln!(
                    "[spot][android] failed to register floating window methods for {}: {:?}",
                    class_name, err
                );
            }
        }
    }
}

#[cfg(target_os = "android")]
pub fn set_floating_window_scene<T: crate::Spot + 'static>() {
    let _ = FLOATING_SCENE_FACTORY.set(Box::new(|ctx| Box::new(T::initialize(ctx))));
}

#[cfg(target_os = "android")]
pub(crate) fn get_floating_scene_factory() -> Option<&'static SceneFactory> {
    FLOATING_SCENE_FACTORY.get()
}

#[cfg(target_os = "android")]
pub(crate) fn take_floating_surface() -> Option<jni::objects::GlobalRef> {
    match FLOATING_SURFACE.lock() {
        Ok(mut guard) => guard.take(),
        Err(err) => {
            eprintln!("[spot][android] failed to lock floating surface: {}", err);
            None
        }
    }
}

#[cfg(target_os = "android")]
pub fn on_surface_created(env: &jni::JNIEnv, surface: jni::objects::JObject) {
    eprintln!("[spot][android] on_surface_created called");
    let Ok(global_ref) = env.new_global_ref(surface) else {
        eprintln!("[spot][android] failed to create global ref for floating surface");
        return;
    };
    match FLOATING_SURFACE.lock() {
        Ok(mut guard) => *guard = Some(global_ref),
        Err(err) => eprintln!("[spot][android] failed to lock floating surface: {}", err),
    }
}

#[cfg(target_os = "android")]
pub fn on_surface_destroyed() {
    match FLOATING_SURFACE.lock() {
        Ok(mut guard) => *guard = None,
        Err(err) => eprintln!("[spot][android] failed to lock floating surface: {}", err),
    }
}

#[cfg(target_os = "android")]
extern "system" fn native_on_floating_surface_created(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    surface: jni::objects::JObject,
) {
    on_surface_created(&env, surface);
}

#[cfg(target_os = "android")]
extern "system" fn native_on_floating_surface_destroyed(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
) {
    on_surface_destroyed();
}

#[cfg(target_os = "android")]
pub fn set_floating_window_service(class_name: &str) {
    let _ = FLOATING_SERVICE_CLASS.set(class_name.to_string());

    // Attempt dynamic registration if JVM and ACTIVITY are already initialized
    if let Some(jvm) = JVM.get() {
        if ACTIVITY.get().is_some() {
            let Ok(mut env) = jvm.attach_current_thread() else {
                eprintln!(
                    "[spot][android] failed to attach thread for floating service registration"
                );
                return;
            };
            if let Err(err) = register_floating_window_methods(&mut env, class_name) {
                eprintln!(
                    "[spot][android] failed to register floating window methods for {}: {:?}",
                    class_name, err
                );
            }
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) fn floating_window_service_class() -> Option<&'static str> {
    FLOATING_SERVICE_CLASS.get().map(|s| s.as_str())
}

#[cfg(target_os = "android")]
pub fn start_service(class_name: &str) {
    let Some(jvm) = JVM.get() else {
        return;
    };
    let Some(activity_ref) = ACTIVITY.get() else {
        return;
    };

    let Ok(mut env) = jvm.attach_current_thread() else {
        eprintln!("[spot][android] failed to attach thread for start_service");
        return;
    };
    let activity = activity_ref.as_obj();

    let Ok(intent_class) = find_class(&mut env, "android/content/Intent") else {
        eprintln!("[spot][android] failed to resolve Intent class");
        return;
    };
    let Ok(service_class) = find_class(&mut env, class_name) else {
        eprintln!(
            "[spot][android] failed to resolve service class {}",
            class_name
        );
        return;
    };

    // new Intent(activity, service_class)
    let Ok(intent) = env.new_object(
        intent_class,
        "(Landroid/content/Context;Ljava/lang/Class;)V",
        &[(&activity).into(), (&service_class).into()],
    ) else {
        eprintln!(
            "[spot][android] failed to create intent for service {}",
            class_name
        );
        return;
    };

    // Context.startService(intent) or Context.startForegroundService(intent)
    let Ok(version_class) = find_class(&mut env, "android/os/Build$VERSION") else {
        eprintln!("[spot][android] failed to resolve Build.VERSION");
        return;
    };
    let Ok(sdk_int) = env.get_static_field(version_class, "SDK_INT", "I") else {
        eprintln!("[spot][android] failed to read SDK_INT");
        return;
    };
    let Ok(sdk_int) = sdk_int.i() else {
        eprintln!("[spot][android] SDK_INT field had unexpected type");
        return;
    };

    if sdk_int >= 26 {
        if let Err(err) = env.call_method(
            &activity,
            "startForegroundService",
            "(Landroid/content/Intent;)Landroid/content/ComponentName;",
            &[(&intent).into()],
        ) {
            eprintln!(
                "[spot][android] startForegroundService failed for {}: {:?}",
                class_name, err
            );
        }
    } else {
        if let Err(err) = env.call_method(
            &activity,
            "startService",
            "(Landroid/content/Intent;)Landroid/content/ComponentName;",
            &[(&intent).into()],
        ) {
            eprintln!(
                "[spot][android] startService failed for {}: {:?}",
                class_name, err
            );
        }
    }
}

#[cfg(target_os = "android")]
pub fn stop_service(class_name: &str) {
    let Some(jvm) = JVM.get() else {
        return;
    };
    let Some(activity_ref) = ACTIVITY.get() else {
        return;
    };

    let Ok(mut env) = jvm.attach_current_thread() else {
        eprintln!("[spot][android] failed to attach thread for stop_service");
        return;
    };
    let activity = activity_ref.as_obj();

    let Ok(intent_class) = find_class(&mut env, "android/content/Intent") else {
        eprintln!("[spot][android] failed to resolve Intent class");
        return;
    };
    let Ok(service_class) = find_class(&mut env, class_name) else {
        eprintln!(
            "[spot][android] failed to resolve service class {}",
            class_name
        );
        return;
    };

    let Ok(intent) = env.new_object(
        intent_class,
        "(Landroid/content/Context;Ljava/lang/Class;)V",
        &[(&activity).into(), (&service_class).into()],
    ) else {
        eprintln!(
            "[spot][android] failed to create stop-service intent for {}",
            class_name
        );
        return;
    };

    if let Err(err) = env.call_method(
        &activity,
        "stopService",
        "(Landroid/content/Intent;)Z",
        &[(&intent).into()],
    ) {
        eprintln!(
            "[spot][android] stopService failed for {}: {:?}",
            class_name, err
        );
    }
}

#[cfg(target_os = "android")]
pub fn current_local_epoch_day() -> Option<u64> {
    let jvm = JVM.get()?;
    let _activity_ref = ACTIVITY.get()?;
    let mut env = jvm.attach_current_thread().ok()?;

    let local_date_class = find_class(&mut env, "java/time/LocalDate").ok()?;
    let today = env
        .call_static_method(&local_date_class, "now", "()Ljava/time/LocalDate;", &[])
        .and_then(|value| value.l())
        .ok()?;

    env.call_method(&today, "toEpochDay", "()J", &[])
        .and_then(|value| value.j())
        .ok()
        .map(|day| day.max(0) as u64)
}

#[cfg(target_os = "android")]
pub fn set_floating_window_enabled(_enabled: bool) {
    // This is now handled by the engine loop checking if service_class is set.
}

#[cfg(target_os = "android")]
fn register_floating_window_methods(
    env: &mut jni::JNIEnv<'_>,
    class_name: &str,
) -> anyhow::Result<()> {
    let class = find_class(env, class_name)?;
    let methods = [
        jni::NativeMethod {
            name: "onFloatingSurfaceCreated".into(),
            sig: "(Landroid/view/Surface;)V".into(),
            fn_ptr: native_on_floating_surface_created as *mut std::ffi::c_void,
        },
        jni::NativeMethod {
            name: "onFloatingSurfaceDestroyed".into(),
            sig: "()V".into(),
            fn_ptr: native_on_floating_surface_destroyed as *mut std::ffi::c_void,
        },
    ];
    env.register_native_methods(class, &methods)?;
    Ok(())
}
