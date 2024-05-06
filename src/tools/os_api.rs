use openxr_sys::pfn::{EnumerateDisplayRefreshRatesFB, GetDisplayRefreshRateFB, RequestDisplayRefreshRateFB};
use openxr_sys::{Result, Session};

use crate::sk::SkInfo;
use crate::system::{BackendOpenXR, Log};
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

pub enum PathEntry {
    File(OsString),
    Dir(OsString),
}

/// Read all the assets of a given assets sub directory
#[cfg(target_os = "android")]
pub fn get_assets(sk_info: Rc<RefCell<SkInfo>>, sub_dir: PathBuf, file_extensions: &Vec<String>) -> Vec<PathEntry> {
    use std::ffi::CString;

    let mut sk_i = sk_info.borrow_mut();
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

/// Read all the assets of a given assets sub directory
#[cfg(not(target_os = "android"))]
pub fn get_assets(_sk_info: Rc<RefCell<SkInfo>>, sub_dir: PathBuf, file_extensions: &Vec<String>) -> Vec<PathEntry> {
    use std::fs::read_dir;
    let sub_dir = sub_dir.to_str().unwrap_or("");
    let mut exts = vec![];
    for extension in file_extensions {
        let extension = extension[1..].to_string();
        exts.push(OsString::from(extension));
    }
    let path_text = env!("CARGO_MANIFEST_DIR").to_owned() + "/assets";
    let path_asset = Path::new(path_text.as_str()).join(sub_dir);
    let mut vec = vec![];

    if path_asset.exists() && path_asset.is_dir() {
        if let Ok(read_dir) = read_dir(path_asset) {
            for file in read_dir.flatten() {
                let path = file.path();

                if file.path().is_file() {
                    if exts.is_empty() {
                        vec.push(PathEntry::File(file.file_name()))
                    } else if let Some(extension) = path.extension() {
                        if exts.is_empty() || exts.contains(&extension.to_os_string()) {
                            vec.push(PathEntry::File(file.file_name()))
                        }
                    }
                }
            }
        }
    }

    vec
}

/// Get the path to internal data directory for Android
#[cfg(target_os = "android")]
pub fn get_internal_path(sk_info: Rc<RefCell<SkInfo>>) -> Option<PathBuf> {
    let mut sk_i = sk_info.borrow_mut();
    let app = sk_i.get_android_app();
    app.internal_data_path()
}

/// Get the path to internal data directory for non android
#[cfg(not(target_os = "android"))]
pub fn get_internal_path(_sk_info: Rc<RefCell<SkInfo>>) -> Option<PathBuf> {
    None
}

/// Get the path to external data directory for Android
#[cfg(target_os = "android")]
pub fn get_external_path(sk_info: Rc<RefCell<SkInfo>>) -> Option<PathBuf> {
    let mut sk_i = sk_info.borrow_mut();
    let app = sk_i.get_android_app();
    app.external_data_path()
}

/// Get the path to internal data directory for non android
#[cfg(not(target_os = "android"))]
pub fn get_external_path(_sk_info: Rc<RefCell<SkInfo>>) -> Option<PathBuf> {
    None
}

/// Open an asset like a file
#[cfg(target_os = "android")]
pub fn open_asset(sk_info: Rc<RefCell<SkInfo>>, asset_path: impl AsRef<Path>) -> Option<File> {
    use std::ffi::CString;

    let mut sk_i = sk_info.borrow_mut();
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
#[cfg(not(target_os = "android"))]
pub fn open_asset(_sk_info: Rc<RefCell<SkInfo>>, asset_path: impl AsRef<Path>) -> Option<File> {
    let path_text = env!("CARGO_MANIFEST_DIR").to_owned() + "/assets";
    let path_asset = Path::new(path_text.as_str()).join(asset_path);
    File::open(path_asset).ok()
}

/// Read the files and eventually the sub directory of a given directory
pub fn get_files(
    _sk_info: Rc<RefCell<SkInfo>>,
    dir: PathBuf,
    file_extensions: &Vec<String>,
    show_other_dirs: bool,
) -> Vec<PathEntry> {
    use std::fs::read_dir;
    let mut exts = vec![];
    for extension in file_extensions {
        exts.push(OsString::from(extension));
    }
    let mut vec = vec![];

    if dir.exists() && dir.is_dir() {
        if let Ok(read_dir) = read_dir(dir) {
            for file in read_dir.flatten() {
                let path = file.path();

                if file.path().is_file() {
                    if exts.is_empty() {
                        vec.push(PathEntry::File(file.file_name()))
                    } else if let Some(extension) = path.extension() {
                        if exts.is_empty() || exts.contains(&extension.to_os_string()) {
                            vec.push(PathEntry::File(file.file_name()))
                        }
                    }
                } else if show_other_dirs && file.path().is_dir() {
                    vec.push(PathEntry::Dir(file.file_name()))
                }
            }
        }
    }
    vec
}

/// Read all the assets of a given assets sub directory
#[cfg(target_os = "android")]
pub fn show_soft_input(show: bool) -> bool {
    use jni::objects::{JObject, JValue};

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
            .call_method(
                im_manager,
                "showSoftInput",
                "(Landroid/view/View;I)Z",
                &[JValue::Object(&JObject::from(view)), 0i32.into()],
            )
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
/// Open an asset like a file
#[cfg(not(target_os = "android"))]
pub fn show_soft_input(_show: bool) -> bool {
    true
}

/// Log the display refresh rate of the device.
/// Not working on Quest 2, says there is 5 values but display an empty array.
pub fn log_display_refresh_rate() {
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        if let Some(rate_display) =
            BackendOpenXR::get_function::<EnumerateDisplayRefreshRatesFB>("xrEnumerateDisplayRefreshRatesFB")
        {
            let input = 0u32;
            let mut output = 5u32;
            let mut array = Vec::with_capacity(10usize);
            match unsafe {
                rate_display(Session::from_raw(BackendOpenXR::session()), input, &mut output, array.as_mut_ptr())
            } {
                Result::SUCCESS => {
                    Log::diag(format!("display rate count {} ", output));
                    Log::diag(format!("display rate array {:?} ", array));
                    //let array = unsafe { slice::from_raw_parts(array_ptr, output as usize) };
                    for (i, rate) in array.iter().enumerate() {
                        Log::diag(format!("display rate {} : {}", i, rate))
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
}

/// Get the current display rate if possible
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

/// set the current display rate if possible.
/// Possible values on Quest are 60 - 72 - 90 - 120
/// returns true if the given value was accepted
pub fn set_display_refresh_rate(rate: f32) -> bool {
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        //>>>>>>>>>>> Set the value
        if let Some(set_new_rate) =
            BackendOpenXR::get_function::<RequestDisplayRefreshRateFB>("xrRequestDisplayRefreshRateFB")
        {
            match unsafe { set_new_rate(Session::from_raw(BackendOpenXR::session()), rate) } {
                Result::SUCCESS => true,
                otherwise => {
                    Log::err(format!("xrRequestDisplayRefreshRateFB failed: {otherwise}"));
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
