use openxr_sys::pfn::{
    EnumerateDisplayRefreshRatesFB, EnumerateEnvironmentBlendModes, GetDisplayRefreshRateFB,
    RequestDisplayRefreshRateFB,
};
use openxr_sys::{EnvironmentBlendMode, Instance, Result, Session, SystemId, ViewConfigurationType};

use crate::sk::SkInfo;
use crate::system::{Backend, BackendOpenXR, BackendXRType, Log};
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

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

pub const USUAL_FPS_SUSPECTS: [i32; 12] = [30, 60, 72, 80, 90, 100, 110, 120, 144, 165, 240, 360];

/// Return and maybe Log all the display refresh rates available.
/// * `with_log` - If true, log the refresh rates available
///
/// ### Examples
/// ```
/// use stereokit_rust::system::BackendOpenXR;
/// BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
///
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::tools::os_api::{get_all_display_refresh_rates,
///                                     get_display_refresh_rate,
///                                     set_display_refresh_rate};
///
/// let refresh_rate_editable = BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate");
/// if refresh_rate_editable {
///     let rates = get_all_display_refresh_rates(true);
///     assert!(!rates.is_empty());
///     let rate = get_display_refresh_rate().unwrap_or(0.0);
///     assert!(rate >= 20.0);
///     assert!(set_display_refresh_rate(rate, true));
///     let rate2 = get_display_refresh_rate().unwrap_or(0.0);
///     assert_eq!(rate, rate2);
/// } else {
///     let rates = get_all_display_refresh_rates(true);
///     // assert!(rates.len(), 5); // with 5 value 0.0
///     let rate = get_display_refresh_rate();
///     assert_eq!(rate , None);
/// }
/// ```
pub fn get_all_display_refresh_rates(with_log: bool) -> Vec<f32> {
    let mut array = [0.0; 40];
    let mut count = 5u32;
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        if let Some(rate_display) =
            BackendOpenXR::get_function::<EnumerateDisplayRefreshRatesFB>("xrEnumerateDisplayRefreshRatesFB")
        {
            match unsafe {
                rate_display(Session::from_raw(BackendOpenXR::session()), 0, &mut count, array.as_mut_ptr())
            } {
                Result::SUCCESS => {
                    if count > 40 {
                        count = 40
                    }
                    match unsafe {
                        rate_display(Session::from_raw(BackendOpenXR::session()), count, &mut count, array.as_mut_ptr())
                    } {
                        Result::SUCCESS => {
                            if with_log {
                                Log::info(format!("There are {count} display rate:"));
                                for (i, iter) in array.iter().enumerate() {
                                    if i >= count as usize {
                                        break;
                                    }
                                    Log::info(format!("   {iter:?} "));
                                }
                            }
                        }
                        otherwise => {
                            Log::err(format!("xrEnumerateDisplayRefreshRatesFB failed: {otherwise}"));
                        }
                    }
                }
                otherwise => {
                    Log::err(format!("xrEnumerateDisplayRefreshRatesFB failed: {otherwise}"));
                }
            }
        } else {
            Log::err("xrEnumerateDisplayRefreshRatesFB binding function error !")
        }
    }
    array[0..(count as usize)].into()
}

/// Get the display rates available from the given list. See [`USUAL_FPS_SUSPECTS`])
/// * `fps_to_get` - The list of fps to test.
/// * `with_log` - If true, will log the available rates.
///
/// see also [`get_all_display_refresh_rates`]
pub fn get_display_refresh_rates(fps_to_get: &[i32], with_log: bool) -> Vec<f32> {
    let default_refresh_rate = get_display_refresh_rate();
    let mut available_rates = vec![];
    for rate in fps_to_get {
        if set_display_refresh_rate(*rate as f32, false) {
            available_rates.push(*rate as f32);
        }
    }
    if let Some(rate) = default_refresh_rate {
        set_display_refresh_rate(rate, with_log);
    }
    if with_log {
        Log::info(format!("There are {} display rate from the given selection:", available_rates.len()));
        for iter in &available_rates {
            Log::info(format!("   {iter:?} "));
        }
    }

    available_rates
}

/// Get the current display rate if possible.
///
/// see also [`set_display_refresh_rate`]
/// see example in [`get_all_display_refresh_rates`]
pub fn get_display_refresh_rate() -> Option<f32> {
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        if let Some(get_default_rate) =
            BackendOpenXR::get_function::<GetDisplayRefreshRateFB>("xrGetDisplayRefreshRateFB")
        {
            let mut default_rate = 0.0;
            match unsafe { get_default_rate(Session::from_raw(BackendOpenXR::session()), &mut default_rate) } {
                Result::SUCCESS => Some(default_rate),
                otherwise => {
                    Log::err(format!("xrGetDisplayRefreshRateFB failed: {otherwise}"));
                    None
                }
            }
        } else {
            Log::err("xrRequestDisplayRefreshRateFB binding function error !");
            None
        }
    } else {
        None
    }
}

/// Set the current display rate if possible.
/// Possible values on Quest and WiVRn are 60 - 80 - 72 - 80 - 90 - 120
/// returns true if the given value was accepted
/// * `rate` - the rate to set
/// * `with_log` - if true, will log the error if the rate was not accepted.
///
/// see example in [`get_all_display_refresh_rates`]
pub fn set_display_refresh_rate(rate: f32, with_log: bool) -> bool {
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        //>>>>>>>>>>> Set the value
        if let Some(set_new_rate) =
            BackendOpenXR::get_function::<RequestDisplayRefreshRateFB>("xrRequestDisplayRefreshRateFB")
        {
            match unsafe { set_new_rate(Session::from_raw(BackendOpenXR::session()), rate) } {
                Result::SUCCESS => true,
                otherwise => {
                    if with_log {
                        Log::err(format!("xrRequestDisplayRefreshRateFB failed: {otherwise}"));
                    }
                    false
                }
            }
        } else {
            Log::err("xrRequestDisplayRefreshRateFB binding function error !");
            false
        }
    } else {
        false
    }
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
                                Log::info(format!("There are {count} env blend modes:"));
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
                                Log::err(format!("xrEnumerateEnvironmentBlendModes failed: {otherwise}"));
                            }
                        }
                    }
                }
            }
            otherwise => {
                if with_log {
                    Log::err(format!("xrEnumerateEnvironmentBlendModes failed: {otherwise}"));
                }
            }
        }
    } else {
        Log::err("xrEnumerateEnvironmentBlendModes binding function error !");
    }
    modes[0..(count as usize)].into()
}
