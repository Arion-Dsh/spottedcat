#[cfg(target_os = "android")]
use std::sync::OnceLock;
#[cfg(target_os = "android")]
use std::sync::Mutex;
#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[cfg(target_os = "android")]
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();
#[cfg(target_os = "android")]
static JVM: OnceLock<jni::JavaVM> = OnceLock::new();
static ACTIVITY: OnceLock<jni::objects::GlobalRef> = OnceLock::new();
static FLOATING_SERVICE_CLASS: OnceLock<String> = OnceLock::new();
static FLOATING_SURFACE: Mutex<Option<jni::objects::GlobalRef>> = Mutex::new(None);
static FLOATING_SCENE_FACTORY: OnceLock<crate::SceneFactory> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn get_jvm() -> Option<&'static jni::JavaVM> {
    JVM.get()
}

#[cfg(target_os = "android")]
pub fn get_activity() -> Option<&'static jni::objects::GlobalRef> {
    ACTIVITY.get()
}


#[cfg(target_os = "android")]
fn find_class<'a>(env: &mut jni::JNIEnv<'a>, class_name: &str) -> jni::errors::Result<jni::objects::JClass<'a>> {
    // If it starts with a known system class, using find_class is fine
    if class_name.starts_with("android/") || class_name.starts_with("java/") {
        return env.find_class(class_name);
    }

    // For app classes, we must use the app's ClassLoader because we might be on a background thread
    let activity = ACTIVITY.get().expect("ACTIVITY not initialized").as_obj();
    let class_loader = env.call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?.l()?;
    
    let class_name_java = env.new_string(class_name.replace('/', "."))?;
    let class_obj = env.call_method(
        class_loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[(&class_name_java).into()],
    )?.l()?;

    Ok(class_obj.into())
}

#[cfg(target_os = "android")]
pub fn init(app: AndroidApp) {
    let _ = ANDROID_APP.set(app.clone());

    unsafe {
        let vm = jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _).unwrap();
        let activity = jni::objects::JObject::from_raw(app.activity_as_ptr() as *mut _);
        let _ = JVM.set(vm);
        let mut env = JVM.get().unwrap().attach_current_thread().unwrap();
        let _ = ACTIVITY.set(env.new_global_ref(activity).unwrap());

        // If service class was already set, register it now
        if let Some(class_name) = floating_window_service_class() {
            if let Ok(class) = find_class(&mut env, class_name) {
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
                env.register_native_methods(class, &methods).expect("Failed to register native methods for floating window service");
            }
        }
    }
}


#[cfg(target_os = "android")]
pub fn set_floating_window_scene<T: crate::Spot + 'static>() {
    let _ = FLOATING_SCENE_FACTORY.set(Box::new(|ctx| Box::new(T::initialize(ctx))));
}

#[cfg(target_os = "android")]
pub(crate) fn get_floating_scene_factory() -> Option<&'static crate::SceneFactory> {
    FLOATING_SCENE_FACTORY.get()
}

#[cfg(target_os = "android")]
pub(crate) fn take_floating_surface() -> Option<jni::objects::GlobalRef> {
    FLOATING_SURFACE.lock().unwrap().take()
}

#[cfg(target_os = "android")]
pub fn on_surface_created(env: &jni::JNIEnv, surface: jni::objects::JObject) {
    eprintln!("[spot][android] on_surface_created called");
    let global_ref = env.new_global_ref(surface).unwrap();
    *FLOATING_SURFACE.lock().unwrap() = Some(global_ref);
}

#[cfg(target_os = "android")]
pub fn on_surface_destroyed() {
    *FLOATING_SURFACE.lock().unwrap() = None;
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
            let mut env = jvm.attach_current_thread().unwrap();
            if let Ok(class) = find_class(&mut env, class_name) {
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
                env.register_native_methods(class, &methods).expect("Failed to register native methods for floating window service");
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
    let Some(jvm) = JVM.get() else { return; };
    let Some(activity_ref) = ACTIVITY.get() else { return; };

    let mut env = jvm.attach_current_thread().unwrap();
    let activity = activity_ref.as_obj();

    let intent_class = find_class(&mut env, "android/content/Intent").unwrap();
    let service_class = find_class(&mut env, class_name).unwrap();
    
    // new Intent(activity, service_class)
    let intent = env.new_object(
        intent_class,
        "(Landroid/content/Context;Ljava/lang/Class;)V",
        &[(&activity).into(), (&service_class).into()],
    ).unwrap();

    // Context.startService(intent) or Context.startForegroundService(intent)
    let version_class = find_class(&mut env, "android/os/Build$VERSION").unwrap();
    let sdk_int = env.get_static_field(version_class, "SDK_INT", "I").unwrap().i().unwrap();

    if sdk_int >= 26 {
        env.call_method(&activity, "startForegroundService", "(Landroid/content/Intent;)Landroid/content/ComponentName;", &[(&intent).into()]).unwrap();
    } else {
        env.call_method(&activity, "startService", "(Landroid/content/Intent;)Landroid/content/ComponentName;", &[(&intent).into()]).unwrap();
    }
}

#[cfg(target_os = "android")]
pub fn stop_service(class_name: &str) {
    let Some(jvm) = JVM.get() else { return; };
    let Some(activity_ref) = ACTIVITY.get() else { return; };

    let mut env = jvm.attach_current_thread().unwrap();
    let activity = activity_ref.as_obj();

    let intent_class = find_class(&mut env, "android/content/Intent").unwrap();
    let service_class = find_class(&mut env, class_name).unwrap();
    
    let intent = env.new_object(
        intent_class,
        "(Landroid/content/Context;Ljava/lang/Class;)V",
        &[(&activity).into(), (&service_class).into()],
    ).unwrap();

    env.call_method(&activity, "stopService", "(Landroid/content/Intent;)Z", &[(&intent).into()]).unwrap();
}

#[cfg(target_os = "android")]
pub fn set_floating_window_enabled(_enabled: bool) {
    // This is now handled by the engine loop checking if service_class is set.
}
