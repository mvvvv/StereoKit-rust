use openxr_sys::pfn::EnumerateEnvironmentBlendModes;
use openxr_sys::{EnvironmentBlendMode, Handle, Instance, Result, SystemId, ViewConfigurationType};
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::sk::SkInfo;
use crate::system::{Backend, BackendOpenXR, BackendXRType, Log};

/// When browsing files because of Android we need this API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEntry {
    File(OsString),
    Dir(OsString),
}

/// For bin tools only
pub fn get_shaders_source_dir() -> String {
    std::env::var("SK_RUST_SHADERS_SOURCE_DIR").unwrap_or("shaders_src".into())
}

/// Where sks shaders are store under assets dir. For bin tools and none android exe. For Android use app.asset_manager()
pub fn get_shaders_sks_dir() -> String {
    std::env::var("SK_RUST_SHADERS_SKS_DIR").unwrap_or("shaders".into())
}

/// For bin tools and non android exe. For Android use app.asset_manager()
pub fn get_assets_dir() -> String {
    std::env::var("SK_RUST_ASSETS_DIR").unwrap_or("assets".into())
}

/// Read all the assets of a given assets sub directory.
/// * `sk_info` - The SkInfo smart pointer
/// * `sub_dir` - The sub directory of the assets directory.
/// * `file_extensions` - The file extensions to filter by.
///
/// Returns a vector of PathEntry.
#[cfg(target_os = "android")]
pub fn get_assets(
    sk_info: &Option<Rc<RefCell<SkInfo>>>,
    sub_dir: PathBuf,
    file_extensions: &Vec<String>,
) -> Vec<PathEntry> {
    use std::ffi::CString;

    if sk_info.is_none() {
        Log::err("get_assets, sk_info is None");
        return vec![];
    }

    let sk_i = sk_info.as_ref().unwrap().borrow_mut();
    let app = sk_i.get_android_app();
    let mut exts = vec![];
    for extension in file_extensions {
        let extension = extension[1..].to_string();
        exts.push(OsString::from(extension));
    }
    let mut vec = vec![];
    if let Ok(cstring) = CString::new(sub_dir.to_str().unwrap_or("Error!!!")) {
        if let Some(asset_dir) = app.asset_manager().open_dir(cstring.as_c_str()) {
            for entry in asset_dir {
                if let Ok(entry_string) = entry.into_string() {
                    let path = PathBuf::from(entry_string.clone());

                    if exts.is_empty() {
                        if let Some(file_name) = path.file_name() {
                            vec.push(PathEntry::File(file_name.into()))
                        } else {
                            Log::err(format!("get_assets, path {:?} don't have a file_name", path));
                        }
                    } else if let Some(extension) = path.extension() {
                        if exts.contains(&extension.to_os_string()) {
                            if let Some(file_name) = path.file_name() {
                                vec.push(PathEntry::File(file_name.into()))
                            }
                        }
                    }
                }
            }
        }
    }
    vec
}

/// Read all the assets of a given assets sub directory.
/// * `sk_info` - The SkInfo smart pointer
/// * `sub_dir` - The sub directory of the assets directory.
/// * `file_extensions` - The file extensions to filter by.
///
/// Returns a vector of PathEntry.
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{tools::os_api::{get_assets, PathEntry}, include_asset_tree};
///
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// const ASSET_DIR: &[&str] = include_asset_tree!("assets");
///
/// let mut file_found = false;
/// let exts = vec![".png".into(), ".jpg".into(), ".jpeg".into()];
/// for dir_name_str in ASSET_DIR {
///     let dir_name_str = if dir_name_str.starts_with("assets/") {
///         &dir_name_str[7..] // remove "assets/" from the path
///     } else {
///         &dir_name_str[6..]  // remove "assets" from the path
///     };
///     println!("{} :", dir_name_str);
///     let mut asset_sub_dir = std::path::PathBuf::new();
///     asset_sub_dir.push(dir_name_str);
///     for file in get_assets(&sk_info, asset_sub_dir, &exts) {
///         println!("--- {:?}", file);
///         if let PathEntry::File(file) = file {
///             if file.into_string().unwrap_or_default()
///                    .ends_with("log_viewer.png") { file_found = true}
///         }
///     }
/// }
/// assert!(file_found);
/// ```
#[cfg(not(target_os = "android"))]
pub fn get_assets(
    _sk_info: &Option<Rc<RefCell<SkInfo>>>,
    sub_dir: PathBuf,
    file_extensions: &Vec<String>,
) -> Vec<PathEntry> {
    use std::{env, fs::read_dir};

    let sub_dir = sub_dir.to_str().unwrap_or("");
    let mut exts = vec![];
    for extension in file_extensions {
        let extension = extension[1..].to_string();
        exts.push(OsString::from(extension));
    }

    let path_text = env::current_dir().unwrap().to_owned().join(get_assets_dir());
    let path_asset = path_text.join(sub_dir);
    let mut vec = vec![];

    if path_asset.exists() {
        if path_asset.is_dir() {
            match read_dir(&path_asset) {
                Ok(read_dir) => {
                    for file in read_dir.flatten() {
                        let path = file.path();

                        if file.path().is_file() {
                            if exts.is_empty() {
                                vec.push(PathEntry::File(file.file_name()))
                            } else if let Some(extension) = path.extension()
                                && (exts.is_empty() || exts.contains(&extension.to_os_string()))
                            {
                                vec.push(PathEntry::File(file.file_name()))
                            }
                        }
                    }
                }
                Err(err) => {
                    Log::diag(format!("Unable to read {path_asset:?}: {err}"));
                }
            }
        } else {
            Log::diag(format!("{path_asset:?} is not a dir"));
        }
    } else {
        Log::diag(format!("{path_asset:?} do not exists"));
    }

    vec
}

/// Get the path to internal data directory for Android
#[cfg(target_os = "android")]
pub fn get_internal_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    if sk_info.is_none() {
        Log::err("get_internal_path, sk_info is None");
        return None;
    }

    let sk_i = sk_info.as_ref().unwrap().borrow_mut();
    let app = sk_i.get_android_app();
    app.internal_data_path()
}

/// Get the path to external data directory for non android
#[cfg(not(target_os = "android"))]
pub fn get_internal_path(_sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    None
}

/// Get the path to external data directory for Android
#[cfg(target_os = "android")]
pub fn get_external_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    if sk_info.is_none() {
        Log::err("get_external_path, sk_info is None");
        return None;
    }

    let sk_i = sk_info.as_ref().unwrap().borrow_mut();
    let app = sk_i.get_android_app();
    app.external_data_path()
}

/// Get the path to external data directory for non android (assets)
#[cfg(not(target_os = "android"))]
pub fn get_external_path(_sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let path_assets = env::current_dir().unwrap().join(get_assets_dir());
    Some(path_assets)
}

/// Open an asset like a file
#[cfg(target_os = "android")]
pub fn open_asset(sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<File> {
    use std::ffi::CString;

    if sk_info.is_none() {
        Log::err("open_asset, sk_info is None");
        return None;
    }

    let sk_i = sk_info.as_ref().unwrap().borrow_mut();
    let app = sk_i.get_android_app();

    if let Ok(cstring) = CString::new(asset_path.as_ref().to_str().unwrap_or("Error!!!")) {
        if let Some(asset) = app.asset_manager().open(cstring.as_c_str()) {
            if let Ok(o_file_desc) = asset.open_file_descriptor() {
                Some(File::from(o_file_desc.fd))
            } else {
                Log::err(format!("open_asset, {:?} cannot get a new file_descriptor", asset_path.as_ref()));
                None
            }
        } else {
            Log::err(format!("open_asset, path {:?} cannot be a opened", asset_path.as_ref()));
            None
        }
    } else {
        Log::err(format!("open_asset, path {:?} cannot be a cstring", asset_path.as_ref()));
        None
    }
}

/// Open an asset like a file
/// * `sk_info` - The SkInfo smart pointer
/// * `asset_path` - The path to the asset.
///
/// Returns a File if the asset was opened successfully, None otherwise.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::os_api::open_asset;
/// use std::io::Read;
///
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// let asset_path = "textures/readme.md";
///
/// let mut file = open_asset(&sk_info, asset_path).expect("File readme should be opened");
///
/// let mut buffer = String::new();
/// file.read_to_string(&mut buffer).expect("File readme should be read");
/// assert!(buffer.starts_with("# Images"));
/// ```
#[cfg(not(target_os = "android"))]
pub fn open_asset(_sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<File> {
    use std::env;

    let path_assets = env::current_dir().unwrap().join(get_assets_dir());
    let path_asset = path_assets.join(asset_path);
    File::open(path_asset).ok()
}

/// Open and read an asset like a file
#[cfg(target_os = "android")]
pub fn read_asset(sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<Vec<u8>> {
    use std::ffi::CString;

    if sk_info.is_none() {
        Log::err("open_asset, sk_info is None");
        return None;
    }

    let sk_i = sk_info.as_ref().unwrap().borrow_mut();
    let app = sk_i.get_android_app();

    if let Ok(cstring) = CString::new(asset_path.as_ref().to_str().unwrap_or("Error!!!")) {
        if let Some(mut asset) = app.asset_manager().open(cstring.as_c_str()) {
            if let Ok(o_buffer) = asset.buffer() {
                Some(o_buffer.to_vec())
            } else {
                Log::err(format!("open_asset, {:?} cannot get the buffer", asset_path.as_ref()));
                None
            }
        } else {
            Log::err(format!("open_asset, path {:?} cannot be a opened", asset_path.as_ref()));
            None
        }
    } else {
        Log::err(format!("open_asset, path {:?} cannot be a cstring", asset_path.as_ref()));
        None
    }
}

/// Open and read an asset like a file
/// * `sk_info` - The SkInfo smart pointer
/// * `asset_path` - The path to the asset.
///
/// Returns a File if the asset was opened successfully, None otherwise.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::os_api::read_asset;
/// use std::io::Read;
///
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// let asset_path = "textures/readme.md";
///
/// let buffer = read_asset(&sk_info, asset_path).expect("File readme should be readable");
/// assert!(buffer.starts_with(b"# Images"));
/// ```
#[cfg(not(target_os = "android"))]
pub fn read_asset(_sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<Vec<u8>> {
    use std::{env, io::Read};

    let path_assets = env::current_dir().unwrap().join(get_assets_dir());
    let path_asset = path_assets.join(&asset_path);
    let mut fd = match File::open(path_asset).ok() {
        Some(file) => file,
        None => {
            Log::err(format!("open_asset, path {:?} cannot be opened", asset_path.as_ref()));
            return None;
        }
    };
    let mut o_buffer = vec![];
    match fd.read_to_end(&mut o_buffer) {
        Ok(_) => Some(o_buffer),
        Err(err) => {
            Log::err(format!("open_asset, path {:?} cannot be read: {}", asset_path.as_ref(), err));
            None
        }
    }
}

/// Read the files and eventually the sub directory of a given directory
/// * `_sk_info` - The SkInfo smart pointer
/// * `dir` - The directory to read.
/// * `file_extensions` - The file extensions to filter by.
/// * `show_other_dirs` - If true, the sub directories will be shown.
///
/// Returns a vector of PathEntry.
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::os_api::{get_files, PathEntry};
///
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// let mut file_found = false;
/// let mut dir_found = false;
/// let exts = vec![".png".into(), ".jpg".into(), ".jpeg".into()];
/// let mut asset_sub_dir = std::path::PathBuf::new();
/// asset_sub_dir.push("assets/textures");
/// for file in get_files(&sk_info, asset_sub_dir, &exts, true) {
///     println!("--- {:?}", file);
///     match file {
///         PathEntry::File(file) => {
///             if file.into_string().unwrap_or_default()
///                    .ends_with("log_viewer.jpeg") { file_found = true }
///         }
///         PathEntry::Dir(dir) => {
///             if dir.into_string().unwrap_or_default()
///                   .ends_with("water") { dir_found = true }
///         }
///     }
/// }
/// assert!(file_found);
/// assert!(dir_found);
/// ```
pub fn get_files(
    _sk_info: &Option<Rc<RefCell<SkInfo>>>,
    dir: PathBuf,
    file_extensions: &Vec<String>,
    show_sub_dirs: bool,
) -> Vec<PathEntry> {
    use std::fs::read_dir;
    let mut exts = vec![];
    for extension in file_extensions {
        let extension = extension[1..].to_string();
        exts.push(OsString::from(extension));
    }
    let mut vec = vec![];

    if dir.exists()
        && dir.is_dir()
        && let Ok(read_dir) = read_dir(dir)
    {
        for file in read_dir.flatten() {
            let path = file.path();

            if file.path().is_file() {
                if exts.is_empty() {
                    vec.push(PathEntry::File(file.file_name()))
                } else if let Some(extension) = path.extension()
                    && (exts.is_empty() || exts.contains(&extension.to_os_string()))
                {
                    vec.push(PathEntry::File(file.file_name()))
                }
            } else if show_sub_dirs && file.path().is_dir() {
                vec.push(PathEntry::Dir(file.file_name()))
            }
        }
    }
    vec
}

/// Open winit IME keyboard. Does nothing on Quest
#[cfg(target_os = "android")]
pub fn show_soft_input_ime(sk_info: &Option<Rc<RefCell<SkInfo>>>, show: bool) -> bool {
    if sk_info.is_none() {
        Log::err("show_soft_input_ime, sk_info is None");
        return false;
    }

    let sk_i = sk_info.as_ref().unwrap().borrow_mut();
    let app = sk_i.get_android_app();
    if show {
        app.show_soft_input(false);
    } else {
        app.hide_soft_input(false);
    }
    true
}
/// Open nothing has we don't have a winit IME keyboard
#[cfg(not(target_os = "android"))]
pub fn show_soft_input_ime(_sk_info: &Option<Rc<RefCell<SkInfo>>>, _show: bool) -> bool {
    false
}

/// Open Android IMS keyboard. This doesn't work for accentuated characters.
#[cfg(target_os = "android")]
pub fn show_soft_input(show: bool) -> bool {
    use jni::objects::JValue;

    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm() as _) } {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no vm !! : {:?}", e));
            return false;
        }
    };
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
    let mut env = match vm.attach_current_thread() {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no env !! : {:?}", e));
            return false;
        }
    };

    let class_ctxt = match env.find_class("android/content/Context") {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no class_ctxt !! : {:?}", e));
            return false;
        }
    };
    let ims = match env.get_static_field(class_ctxt, "INPUT_METHOD_SERVICE", "Ljava/lang/String;") {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no ims !! : {:?}", e));
            return false;
        }
    };

    let im_manager = match env
        .call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[ims.borrow()])
        .unwrap()
        .l()
    {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no im_manager !! : {:?}", e));
            return false;
        }
    };

    let jni_window = match env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l() {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no jni_window !! : {:?}", e));
            return false;
        }
    };

    let view = match env.call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[]).unwrap().l() {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("virtual_kbd : no view !! : {:?}", e));
            return false;
        }
    };

    if show {
        let result = env
            .call_method(im_manager, "showSoftInput", "(Landroid/view/View;I)Z", &[JValue::Object(&view), 0i32.into()])
            .unwrap()
            .z()
            .unwrap();
        result
    } else {
        let window_token = env.call_method(view, "getWindowToken", "()Landroid/os/IBinder;", &[]).unwrap().l().unwrap();
        let jvalue_window_token = jni::objects::JValueGen::Object(&window_token);

        let result = env
            .call_method(
                im_manager,
                "hideSoftInputFromWindow",
                "(Landroid/os/IBinder;I)Z",
                &[jvalue_window_token, 0i32.into()],
            )
            .unwrap()
            .z()
            .unwrap();
        result
    }
}

/// Open nothing has we don't have a virtual keyboard
#[cfg(not(target_os = "android"))]
pub fn show_soft_input(_show: bool) -> bool {
    false
}

/// Open the default browser. Adapted from https://github.com/amodm/webbrowser-rs
#[cfg(target_os = "android")]
pub fn launch_browser_android(url: &str) -> bool {
    use jni::objects::{JObject, JValue};

    Log::diag(format!("launch_browser_android: Attempting to open URL: {}", url));

    // Create a VM for executing Java calls
    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm() as _) } {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no vm !! : {:?}", e));
            return false;
        }
    };

    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
    let mut env = match vm.attach_current_thread() {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no env !! : {:?}", e));
            return false;
        }
    };

    // Create ACTION_VIEW object
    let intent_class = match env.find_class("android/content/Intent") {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no intent_class !! : {:?}", e));
            return false;
        }
    };
    let action_view = match env.get_static_field(&intent_class, "ACTION_VIEW", "Ljava/lang/String;") {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no action_view !! : {:?}", e));
            return false;
        }
    };

    // Create Uri object
    let uri_class = match env.find_class("android/net/Uri") {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no uri_class !! : {:?}", e));
            return false;
        }
    };
    let url_string = match env.new_string(url) {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no url_string !! : {:?}", e));
            return false;
        }
    };
    let uri = match env
        .call_static_method(
            &uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&JObject::from(url_string))],
        )
        .unwrap()
        .l()
    {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no uri !! : {:?}", e));
            return false;
        }
    };

    // Create new ACTION_VIEW intent with the uri
    let intent = match env.alloc_object(&intent_class) {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("launch_browser_android: no intent !! : {:?}", e));
            return false;
        }
    };
    if let Err(e) = env.call_method(
        &intent,
        "<init>",
        "(Ljava/lang/String;Landroid/net/Uri;)V",
        &[action_view.borrow(), JValue::Object(&uri)],
    ) {
        Log::err(format!("launch_browser_android: intent init failed !! : {:?}", e));
        return false;
    }

    // Start the intent activity.
    match env.call_method(&activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)]) {
        Ok(_) => {
            // Just clear any pending exceptions without detailed analysis
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
                Log::err("launch_browser_android: Activity exception occurred (cleared)");
                return false;
            }
            true
        }
        Err(e) => {
            Log::err(format!("launch_browser_android: startActivity failed: {} | URL: {}", e, url));
            // Clear any pending exceptions without trying to read them
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            false
        }
    }
}

/// Open nothing has we aren't on Android
#[cfg(not(target_os = "android"))]
pub fn launch_browser_android(_url: &str) -> bool {
    false
}

/// Supported system deep link actions for Meta Quest following official Meta specifications
#[derive(Debug, Clone, PartialEq)]
pub enum SystemAction {
    /// Open the browser with a URL
    /// Intent: systemux://browser, URI: [any valid URL]
    Browser { url: String },
    /// Open the store
    /// Intent: systemux://store, URI: \[none\] for front page or /item/\[ID\] for specific app
    Store { app_id: Option<String> },
    /// Open settings
    /// Intent: systemux://settings, URI options:
    /// - \[none\]: Main settings page
    /// - /hands: Hand tracking settings
    /// - /system: System settings
    /// - /privacy: Privacy settings
    /// - /controllers: Controllers settings
    /// - /bluetooth: Bluetooth settings
    /// - /wifi: WiFi settings
    /// - /device: Device settings
    /// - /guardian: Guardian/Boundary settings
    /// - /accounts: Accounts settings
    /// - /notifications: Notifications settings
    /// - /applications?package=com.X.Y: Settings for specific app
    Settings { setting: Option<String> },
    /// Open Files app
    /// Intent: systemux://file-manager, URI: \[none\] for Recents, /media/ for Media tab, /downloads/ for Downloads tab
    FileManager { path: Option<String> },
    /// Open Meta bug reporter
    /// Intent: systemux://bug_report, URI: N/A
    BugReport,
}

/// System Deep Link function for Meta Quest following Meta specifications.
/// This function uses the VR Shell package to trigger various system actions.
///
/// # Arguments
/// * `action` - The system action to perform
///
/// # Examples
/// ```
/// // Open browser with URL
/// system_deep_link(SystemAction::Browser("https://www.oculus.com".to_string()));
///
/// // Open store with app ID
/// system_deep_link(SystemAction::Store("1234567890".to_string()));
///
/// // Open home screen
/// system_deep_link(SystemAction::Home);
///
/// // Custom action
/// system_deep_link(SystemAction::Custom {
///     action: "myaction".to_string(),
///     data: Some("mydata".to_string())
/// });
/// ```
/// System Deep Link function for Meta Quest following Meta specifications.
/// This function uses the VR Shell package to trigger various system actions.
///
/// # Arguments
/// * `action` - The system action to perform
///
/// # Examples
/// ```
/// // Open browser with URL
/// system_deep_link(SystemAction::Browser("https://www.oculus.com".to_string()));
///
/// // Open store with app ID
/// system_deep_link(SystemAction::Store("1234567890".to_string()));
///
/// // Open home screen
/// system_deep_link(SystemAction::Home);
///
/// // Custom action
/// System Deep Link function for Meta Quest following official Meta specifications.
/// This function uses PackageManager.getLaunchIntentForPackage() with intent_data and uri extras
/// as documented in Meta's System Deep Linking guide.
///
/// # Arguments
/// * `action` - The system action to perform
///
/// # Examples
/// ```
/// // Open browser with URL
/// system_deep_link(SystemAction::Browser { url: "https://www.meta.com".to_string() });
///
/// // Open store front page
/// system_deep_link(SystemAction::Store { app_id: None });
///
/// // Open store page for specific app
/// system_deep_link(SystemAction::Store { app_id: Some("1234567890".to_string()) });
///
/// // Open settings main page
/// system_deep_link(SystemAction::Settings { setting: None });
///
/// // Open hand tracking settings
/// system_deep_link(SystemAction::Settings { setting: Some("/hands".to_string()) });
///
/// // Open controllers settings
/// system_deep_link(SystemAction::Settings { setting: Some("/controllers".to_string()) });
///
/// // Open WiFi settings
/// system_deep_link(SystemAction::Settings { setting: Some("/wifi".to_string()) });
///
/// // Open Guardian/Boundary settings
/// system_deep_link(SystemAction::Settings { setting: Some("/guardian".to_string()) });
///
/// // Open settings for specific app
/// system_deep_link(SystemAction::Settings { setting: Some("/applications?package=com.oculus.browser".to_string()) });
///
/// // Open bug reporter
/// system_deep_link(SystemAction::BugReport);
/// ```
#[cfg(target_os = "android")]
pub fn system_deep_link(action: SystemAction) -> bool {
    use crate::system::Log;
    use jni::objects::JValue;

    // Create a VM for executing Java calls
    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm() as _) } {
        Ok(value) => value,
        Err(e) => {
            Log::err(format!("system_deep_link: Failed to get VM: {:?}", e));
            return false;
        }
    };

    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(e) => {
            Log::err(format!("system_deep_link: Failed to attach to JVM thread: {:?}", e));
            return false;
        }
    };

    // Prepare action info according to Meta specifications
    let (intent_data, uri_value, display_data) = match &action {
        SystemAction::Browser { url } => {
            ("systemux://browser", url.clone(), format!("Opening browser with URL: {}", url))
        }
        SystemAction::Store { app_id } => {
            let uri = match app_id {
                Some(id) => format!("/item/{}", id),
                None => String::new(),
            };
            (
                "systemux://store",
                uri,
                format!(
                    "Opening store{}",
                    if app_id.is_some() { format!(" for app: {}", app_id.as_ref().unwrap()) } else { String::new() }
                ),
            )
        }
        SystemAction::Settings { setting } => {
            let uri = setting.as_deref().unwrap_or("");
            (
                "systemux://settings",
                uri.to_string(),
                format!("Opening settings{}", if !uri.is_empty() { format!(": {}", uri) } else { String::new() }),
            )
        }
        SystemAction::FileManager { path } => {
            let uri = path.as_deref().unwrap_or("");
            (
                "systemux://file-manager",
                uri.to_string(),
                format!("Opening file manager{}", if !uri.is_empty() { format!(": {}", uri) } else { String::new() }),
            )
        }
        SystemAction::BugReport => ("systemux://bug_report", String::new(), "Opening bug report".to_string()),
    };

    Log::info(format!("system_deep_link: {}", display_data));
    Log::diag(format!(
        "system_deep_link: Attempting to launch VR Shell with intent_data='{}', uri='{}'",
        intent_data, uri_value
    ));

    // Get PackageManager from context (following Meta specification)
    let package_manager =
        match env.call_method(&activity, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[]) {
            Ok(pm) => match pm.l() {
                Ok(pm_obj) => pm_obj,
                Err(e) => {
                    Log::err(format!("system_deep_link: Failed to extract PackageManager object: {}", e));
                    return false;
                }
            },
            Err(e) => {
                Log::err(format!("system_deep_link: Failed to get PackageManager: {}", e));
                return false;
            }
        };

    // Get launch intent for VR Shell (following Meta specification)
    let package_name = match env.new_string("com.oculus.vrshell") {
        Ok(s) => s,
        Err(e) => {
            Log::err(format!("system_deep_link: Failed to create package name string: {}", e));
            return false;
        }
    };

    let intent = match env.call_method(
        &package_manager,
        "getLaunchIntentForPackage",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&package_name.into())],
    ) {
        Ok(intent_result) => match intent_result.l() {
            Ok(intent_obj) if !intent_obj.is_null() => intent_obj,
            Ok(_) => {
                Log::err("system_deep_link: getLaunchIntentForPackage returned null");
                return false;
            }
            Err(e) => {
                Log::err(format!("system_deep_link: Failed to extract Intent object: {}", e));
                return false;
            }
        },
        Err(e) => {
            Log::err(format!("system_deep_link: Failed to get launch intent: {}", e));
            return false;
        }
    };

    // Add intent_data extra (following Meta specification)
    let intent_data_key = match env.new_string("intent_data") {
        Ok(s) => s,
        Err(e) => {
            Log::err(format!("system_deep_link: Failed to create intent_data key: {}", e));
            return false;
        }
    };

    let intent_data_value = match env.new_string(intent_data) {
        Ok(s) => s,
        Err(e) => {
            Log::err(format!("system_deep_link: Failed to create intent_data value: {}", e));
            return false;
        }
    };

    if let Err(e) = env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&intent_data_key.into()), JValue::Object(&intent_data_value.into())],
    ) {
        Log::err(format!("system_deep_link: Failed to add intent_data extra: {}", e));
        return false;
    }

    // Add uri extra if not empty (following Meta specification)
    if !uri_value.is_empty() {
        let uri_key = match env.new_string("uri") {
            Ok(s) => s,
            Err(e) => {
                Log::err(format!("system_deep_link: Failed to create uri key: {}", e));
                return false;
            }
        };

        let uri_value_string = match env.new_string(&uri_value) {
            Ok(s) => s,
            Err(e) => {
                Log::err(format!("system_deep_link: Failed to create uri value: {}", e));
                return false;
            }
        };

        if let Err(e) = env.call_method(
            &intent,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&uri_key.into()), JValue::Object(&uri_value_string.into())],
        ) {
            Log::err(format!("system_deep_link: Failed to add uri extra: {}", e));
            return false;
        }
    }

    // Start activity (following Meta specification)
    match env.call_method(&activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)]) {
        Ok(_) => {
            // Just clear any pending exceptions without detailed analysis
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
                Log::err("system_deep_link: Activity exception occurred (cleared)");
                return false;
            }
            Log::info(format!("system_deep_link: Successfully executed: {}", display_data));
            true
        }
        Err(e) => {
            Log::err(format!(
                "system_deep_link: Failed to start activity: {} | Action: {:?} | Intent data: {} | URI: {}",
                e, action, intent_data, uri_value
            ));
            // Clear any pending exceptions without trying to read them
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            false
        }
    }
}

#[cfg(not(target_os = "android"))]
pub fn system_deep_link(_action: SystemAction) -> bool {
    use crate::system::Log;
    Log::warn("system_deep_link: Not supported on non-Android platforms");
    false
}

/// Get the list of environnement blend_modes available on this device.
/// * `with_log` - if true, will log the available blend modes.
///
/// see also [`crate::util::Device::valid_blend`]
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::{tools::os_api::get_env_blend_modes, util::Device, sk::DisplayBlend};
/// use openxr_sys::EnvironmentBlendMode;
///
/// let blend_modes = get_env_blend_modes(true);
/// if blend_modes.len() > 0 {
///     assert!(blend_modes.contains(&EnvironmentBlendMode::OPAQUE));
///     if blend_modes.contains(&EnvironmentBlendMode::ADDITIVE)
///     || blend_modes.contains(&EnvironmentBlendMode::ALPHA_BLEND)
///     {
///        println!("Passthrough available !!");
///        // we can activate passthrough:
///        Device::display_blend(DisplayBlend::AnyTransparent);
///     }
/// } else {
///     // Simplest way to check if passthrough is available:
///     assert_eq!(Device::valid_blend(DisplayBlend::AnyTransparent), false);
/// }
///
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 0 {
///         // activate passthrough if available
///         Device::display_blend(DisplayBlend::AnyTransparent);
///     } else if iter == 1 {
///         // deactivate passthrough
///         Device::display_blend(DisplayBlend::Opaque);
///     }
/// );
/// ```
pub fn get_env_blend_modes(with_log: bool) -> Vec<EnvironmentBlendMode> {
    //>>>>>>>>>>> Get the env blend mode
    let mut count = 0u32;
    let mut modes = [EnvironmentBlendMode::OPAQUE; 20];
    if Backend::xr_type() != BackendXRType::OpenXR {
        return vec![];
    }
    if let Some(get_modes) =
        BackendOpenXR::get_function::<EnumerateEnvironmentBlendModes>("xrEnumerateEnvironmentBlendModes")
    {
        match unsafe {
            get_modes(
                Instance::from_raw(BackendOpenXR::instance()),
                SystemId::from_raw(BackendOpenXR::system_id()),
                ViewConfigurationType::PRIMARY_STEREO,
                0,
                &mut count,
                modes.as_mut_ptr(),
            )
        } {
            Result::SUCCESS => {
                if with_log {
                    if count > 20 {
                        count = 20
                    }
                    match unsafe {
                        get_modes(
                            Instance::from_raw(BackendOpenXR::instance()),
                            SystemId::from_raw(BackendOpenXR::system_id()),
                            ViewConfigurationType::PRIMARY_STEREO,
                            count,
                            &mut count,
                            modes.as_mut_ptr(),
                        )
                    } {
                        Result::SUCCESS => {
                            if with_log {
                                Log::info(format!("✅ There are {count} env blend modes:"));
                                for (i, iter) in modes.iter().enumerate() {
                                    if i >= count as usize {
                                        break;
                                    }
                                    Log::info(format!("   {iter:?} "));
                                }
                            }
                        }
                        otherwise => {
                            if with_log {
                                Log::err(format!("❌ xrEnumerateEnvironmentBlendModes failed: {otherwise}"));
                            }
                        }
                    }
                }
            }
            otherwise => {
                if with_log {
                    Log::err(format!("❌ xrEnumerateEnvironmentBlendModes failed: {otherwise}"));
                }
            }
        }
    } else {
        Log::err("❌ xrEnumerateEnvironmentBlendModes binding function error !");
    }
    modes[0..(count as usize)].into()
}
