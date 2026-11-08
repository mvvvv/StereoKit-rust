use openxr_sys::pfn::EnumerateEnvironmentBlendModes;
use openxr_sys::{EnvironmentBlendMode, Handle, Instance, Result, SystemId, ViewConfigurationType};
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::sk::SkInfo;
#[cfg(target_os = "android")]
use crate::system::BackendAndroid;
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

    let sk_i = sk_info.as_ref().expect("sk_info should be Some").borrow_mut();
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
/// # sk::Sk::shutdown();
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

    let path_text = env::current_dir().unwrap_or_default().to_owned().join(get_assets_dir());
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

    let sk_i = sk_info.as_ref().expect("sk_info should be Some").borrow_mut();
    let app = sk_i.get_android_app();
    app.internal_data_path()
}

/// Get the path to the application's internal files directory (XDG `data` dir) for Linux.
///
/// This is where an application stores its internal files, following the
/// [XDG Base Directory spec](https://specifications.freedesktop.org/basedir-spec/latest/):
/// `$XDG_DATA_HOME/<app_name>`, or `~/.local/share/<app_name>` when `XDG_DATA_HOME`
/// is not set (or not absolute).
///
/// * `sk_info` - The SkInfo smart pointer, used to get the application name.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::get_internal_path;
///
/// // `~/.local/share/<app_name>` on Linux, `%APPDATA%\<app_name>` on Windows,
/// // `~/Library/Application Support/<app_name>` on macOS.
/// if let Some(internal_dir) = get_internal_path(&None) {
///     println!("App internal files dir: {internal_dir:?}");
/// }
/// ```
#[cfg(target_os = "linux")]
pub fn get_internal_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let base = env::var("XDG_DATA_HOME")
        .ok()
        .filter(|dir| Path::new(dir).is_absolute())
        .map(PathBuf::from)
        .or_else(|| env::var("HOME").ok().map(|home| PathBuf::from(home).join(".local").join("share")))?;

    Some(base.join(app_dir_name(sk_info)))
}

/// Get the path to the application's internal files directory for macOS.
///
/// This is where an application stores its internal files:
/// `~/Library/Application Support/<app_name>` (macOS has no XDG equivalent,
/// settings and data conventionally share the same folder).
///
/// * `sk_info` - The SkInfo smart pointer, used to get the application name.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::get_internal_path;
///
/// // `~/Library/Application Support/<app_name>` on macOS.
/// if let Some(internal_dir) = get_internal_path(&None) {
///     println!("App internal files dir: {internal_dir:?}");
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn get_internal_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let home = env::var("HOME").ok()?;
    Some(PathBuf::from(home).join("Library").join("Application Support").join(app_dir_name(sk_info)))
}

/// Get the path to the application's internal files directory for Windows.
///
/// This is where an application stores its internal files:
/// `%APPDATA%\<app_name>` (i.e. `C:\Users\<user>\AppData\Roaming\<app_name>`),
/// with a `~\AppData\Roaming` fallback if `%APPDATA%` is missing.
///
/// * `sk_info` - The SkInfo smart pointer, used to get the application name.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::get_internal_path;
///
/// // `%APPDATA%\<app_name>` on Windows.
/// if let Some(internal_dir) = get_internal_path(&None) {
///     println!("App internal files dir: {internal_dir:?}");
/// }
/// ```
#[cfg(target_os = "windows")]
pub fn get_internal_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let base = env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .or_else(|| env::var("USERPROFILE").ok().map(|home| PathBuf::from(home).join("AppData").join("Roaming")))?;

    Some(base.join(app_dir_name(sk_info)))
}

/// Get the path to external data directory for Android
#[cfg(target_os = "android")]
pub fn get_external_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    if sk_info.is_none() {
        Log::err("get_external_path, sk_info is None");
        return None;
    }

    let sk_i = sk_info.as_ref().expect("sk_info should be Some").borrow_mut();
    let app = sk_i.get_android_app();
    app.external_data_path()
}

/// Get the path to the application's settings directory (XDG `config` dir) for Linux.
///
/// This is where an application stores its settings, following the
/// [XDG Base Directory spec](https://specifications.freedesktop.org/basedir-spec/latest/):
/// `$XDG_CONFIG_HOME/<app_name>`, or `~/.config/<app_name>` when `XDG_CONFIG_HOME`
/// is not set (or not absolute).
///
/// * `sk_info` - The SkInfo smart pointer, used to get the application name.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::get_external_path;
///
/// // `~/.config/<app_name>` on Linux, `%APPDATA%\<app_name>` on Windows,
/// // `~/Library/Application Support/<app_name>` on macOS.
/// if let Some(settings_dir) = get_external_path(&None) {
///     println!("App settings dir: {settings_dir:?}");
/// }
/// ```
#[cfg(target_os = "linux")]
pub fn get_external_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let base = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|dir| Path::new(dir).is_absolute())
        .map(PathBuf::from)
        .or_else(|| env::var("HOME").ok().map(|home| PathBuf::from(home).join(".config")))?;

    Some(base.join(app_dir_name(sk_info)))
}

/// Get the path to the application's settings directory for macOS.
///
/// This is where an application stores its settings:
/// `~/Library/Application Support/<app_name>` (macOS has no XDG equivalent,
/// settings and data conventionally share the same folder).
///
/// * `sk_info` - The SkInfo smart pointer, used to get the application name.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::get_external_path;
///
/// // `~/Library/Application Support/<app_name>` on macOS.
/// if let Some(settings_dir) = get_external_path(&None) {
///     println!("App settings dir: {settings_dir:?}");
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn get_external_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let home = env::var("HOME").ok()?;
    Some(PathBuf::from(home).join("Library").join("Application Support").join(app_dir_name(sk_info)))
}

/// Get the path to the application's settings directory for Windows.
///
/// This is where an application stores its settings:
/// `%APPDATA%\<app_name>` (i.e. `C:\Users\<user>\AppData\Roaming\<app_name>`),
/// with a `~\AppData\Roaming` fallback if `%APPDATA%` is missing.
///
/// * `sk_info` - The SkInfo smart pointer, used to get the application name.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::get_external_path;
///
/// // `%APPDATA%\<app_name>` on Windows.
/// if let Some(settings_dir) = get_external_path(&None) {
///     println!("App settings dir: {settings_dir:?}");
/// }
/// ```
#[cfg(target_os = "windows")]
pub fn get_external_path(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
    use std::env;

    let base = env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .or_else(|| env::var("USERPROFILE").ok().map(|home| PathBuf::from(home).join("AppData").join("Roaming")))?;

    Some(base.join(app_dir_name(sk_info)))
}

/// Get the name of the application used to build its per-app settings/data folders.
///
/// The name comes, in order, from:
/// 1. the StereoKit settings ([`crate::sk::SkSettings::app_name`]) when `sk_info` is available,
/// 2. the current executable file name,
/// 3. the `"StereoKitApp"` default.
#[cfg(not(target_os = "android"))]
fn app_dir_name(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> String {
    use std::ffi::CStr;

    // 1. The application name from the StereoKit settings.
    if let Some(sk) = sk_info {
        let settings = sk.borrow().get_settings();
        if !settings.app_name.is_null() {
            let name = unsafe { CStr::from_ptr(settings.app_name) }.to_string_lossy().trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // 2. The current executable name.
    if let Ok(exe) = std::env::current_exe()
        && let Some(stem) = exe.file_stem()
    {
        let name = stem.to_string_lossy().trim().to_string();
        if !name.is_empty() {
            return name;
        }
    }

    // 3. The StereoKit default app name.
    "StereoKitApp".to_string()
}

/// The storage locations a file browser can browse.
///
/// On Android they map to the app storage places, on PC (non Android) they are juxtaposed
/// with the standard per-OS application and user folders:
///
/// | Location                      | Android                          | PC                                                     |
/// |-------------------------------|----------------------------------|--------------------------------------------------------|
/// | [`BrowseLocation::Internal`]  | `internal_data_path`             | app internal files dir (`~/.local/share/<app>` on Linux) |
/// | [`BrowseLocation::External`]  | `external_data_path`             | app settings dir (`~/.config/<app>` on Linux)            |
/// | [`BrowseLocation::Documents`] | shared public Documents folder   | user `Documents` folder                                  |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowseLocation {
    /// Android: app internal data path (`/data/data/<package>/files`).
    /// PC: the app's internal files dir — Linux `~/.local/share/<app>`,
    /// macOS `~/Library/Application Support/<app>`, Windows `%APPDATA%\<app>`.
    Internal,
    /// Android: app external data path (`/storage/self/primary/Android/data/<package>/files`).
    /// PC: the app's settings dir — Linux `~/.config/<app>`,
    /// macOS `~/Library/Application Support/<app>`, Windows `%APPDATA%\<app>`.
    #[default]
    External,
    /// Android: shared public Documents folder (`/storage/self/primary/Documents`).
    /// PC: the user's `Documents` folder.
    Documents,
    Pictures,
    Movies,
    Music,
    Downloads,
}

impl BrowseLocation {
    /// The on-disk directory name, used to build the public directory path
    /// (e.g. `/storage/self/primary/<dir>` on Android, `~/<dir>` on PC).
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowseLocation::Internal => "internal",
            BrowseLocation::External => "external",
            BrowseLocation::Documents => "Documents",
            BrowseLocation::Pictures => "Pictures",
            BrowseLocation::Movies => "Movies",
            BrowseLocation::Music => "Music",
            #[cfg(target_os = "android")]
            BrowseLocation::Downloads => "Download",
            #[cfg(not(target_os = "android"))]
            BrowseLocation::Downloads => "Downloads",
        }
    }

    /// Resolve the root directory of a [`BrowseLocation`]:
    /// * Android: `internal_data_path` / `external_data_path` / shared public folders.
    /// * PC: app internal files dir / app settings dir / user public folders.
    ///
    /// * `sk_info` - The SkInfo smart pointer
    /// * `location` - The location to resolve.
    pub fn get_path(&self, sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<PathBuf> {
        match self {
            BrowseLocation::Internal => get_internal_path(sk_info).or_else(|| std::env::current_dir().ok()),
            BrowseLocation::External => get_external_path(sk_info),
            otherwise => get_public_dir(*otherwise),
        }
    }
}

/// Root of the emulated (internal) storage, as returned by
/// `StorageManager.getPrimaryStorageVolume().getDirectory().getAbsolutePath()`.
///
/// That physical path always belongs to the **first** user (`/storage/emulated/0`), even when the
/// app runs as another user, so it must never be used as is on a multi-user device.
#[cfg(any(target_os = "android", test))]
const EMULATED_STORAGE_ROOT_PREFIX: &str = "/storage/emulated/";

/// Per-user alias of the emulated storage root: the kernel resolves it to `/storage/emulated/<user id>`
/// of the user the calling process belongs to, which is the right root on multi-user devices.
#[cfg(any(target_os = "android", test))]
const PER_USER_PRIMARY_STORAGE_ROOT: &str = "/storage/self/primary";

/// Map the physical root of the primary storage volume to the root to use for the current user.
///
/// `StorageVolume.getDirectory()` returns a *physical* path, which always belongs to the first user
/// (`/storage/emulated/0`) for an emulated volume: it is therefore remapped to the per-user alias
/// `/storage/self/primary`, which the kernel resolves to `/storage/emulated/<user id>` of the process.
/// A physical volume (adopted SD card) has no such alias and is kept as is.
#[cfg(any(target_os = "android", test))]
fn per_user_public_root(volume_root: &str) -> PathBuf {
    if volume_root.starts_with(EMULATED_STORAGE_ROOT_PREFIX) {
        PathBuf::from(PER_USER_PRIMARY_STORAGE_ROOT)
    } else {
        PathBuf::from(volume_root)
    }
}

/// Get the path to a shared public folder on Android.
///
/// Resolves the root of the primary storage volume through `StorageManager` (no deprecated API),
/// then appends the [`BrowseLocation`] folder name.
///
/// `StorageVolume.getDirectory()` returns the *physical* emulated path, which always belongs to the
/// first user (`/storage/emulated/0`). So that the result stays valid on multi-user devices, an
/// emulated root is remapped to the per-user alias `/storage/self/primary`, which the kernel resolves
/// to `/storage/emulated/<user id>` of the current process. A physical primary volume (adopted SD
/// card) is used as is, since no alias exists for it.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::{get_public_dir, BrowseLocation};
///
/// // e.g. `/storage/self/primary/Documents`
/// if let Some(documents) = get_public_dir(BrowseLocation::Documents) {
///     println!("Documents folder: {documents:?}");
/// }
/// ```
#[cfg(target_os = "android")]
pub fn get_public_dir(dir_type: BrowseLocation) -> Option<PathBuf> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JString, JValue},
    };

    let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };

    // 2. Appel JNI pour récupérer le chemin racine via StorageManager
    let path_str = vm
        .attach_current_thread(|env| -> jni::errors::Result<String> {
            let context = unsafe { jni::objects::JObject::from_raw(env, BackendAndroid::activity() as _) };
            // StorageManager storageManager = (StorageManager) context.getSystemService(Context.STORAGE_SERVICE);
            let service_name = env.new_string("storage")?;
            let storage_manager = env
                .call_method(
                    &context,
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&JObject::from(service_name))],
                )?
                .l()?;

            if storage_manager.is_null() {
                return Ok(String::new());
            }

            // StorageVolume primaryVolume = storageManager.getPrimaryStorageVolume();
            let primary_volume = env
                .call_method(
                    &storage_manager,
                    jni_str!("getPrimaryStorageVolume"),
                    jni_sig!("()Landroid/os/storage/StorageVolume;"),
                    &[],
                )?
                .l()?;

            if primary_volume.is_null() {
                return Ok(String::new());
            }

            // File rootDir = primaryVolume.getDirectory(); (Android 11 / API 30+)
            let root_file =
                env.call_method(&primary_volume, jni_str!("getDirectory"), jni_sig!("()Ljava/io/File;"), &[])?.l()?;

            if root_file.is_null() {
                return Ok(String::new());
            }

            // String rootPath = rootDir.getAbsolutePath();
            let root_path_obj = env
                .call_method(&root_file, jni_str!("getAbsolutePath"), jni_sig!("()Ljava/lang/String;"), &[])?
                .l()?;

            let java_str = unsafe { JString::from_raw(env, root_path_obj.into_raw() as jni::sys::jstring) };
            Ok(java_str.to_string())
        })
        .ok()?;

    if !path_str.is_empty() {
        // `StorageVolume.getDirectory()` gives the *physical* path of the volume, i.e.
        // `/storage/emulated/0` for the emulated primary storage, which always belongs to the
        // first user. `per_user_public_root` remaps it to the per-user alias
        // `/storage/self/primary` (see its documentation), so multi-user devices resolve the
        // right folder, while a physical primary volume (adopted SD card) is kept as is.
        let root = per_user_public_root(&path_str);
        Log::diag(format!(
            "get_public_dir({:?}) uses root {:?} (primary volume root: {:?})",
            dir_type, root, path_str
        ));
        let path = root.join(dir_type.as_str());
        Log::diag(format!("get_public_dir({:?}) resolved to {:?}", dir_type, path));
        Some(path)
    } else {
        // Fallback si le StorageManager ne répond pas
        Log::diag(format!("get_public_dir({:?}) resolved to uggly default", dir_type));
        Some(PathBuf::from(format!(
            "{}/{}",
            PER_USER_PRIMARY_STORAGE_ROOT,
            dir_type.as_str()
        )))
    }
}

/// Get the path to a shared public folder for non android (user home folders).
///
/// Maps the [`BrowseLocation`] to the corresponding user folder on the PC:
///
/// * [`BrowseLocation::Documents`] -> user `Documents` folder
/// * [`BrowseLocation::Pictures`]  -> user `Pictures`  folder
/// * [`BrowseLocation::Movies`]    -> user `Videos` folder (`Movies` on macOS)
/// * [`BrowseLocation::Music`]     -> user `Music`     folder
/// * [`BrowseLocation::Downloads`] -> user `Downloads` folder
///
/// The home folder resolution is OS dependant (first match wins):
///
/// | OS      | Resolution order                                                                                 |
/// |---------|--------------------------------------------------------------------------------------------------|
/// | Linux   | `XDG_*_DIR` env var, then `${XDG_CONFIG_HOME:-$HOME/.config}/user-dirs.dirs`, then `$HOME/<folder>` |
/// | macOS   | `$HOME/<folder>` (no XDG, the video folder is named `Movies`)                                     |
/// | Windows | `%USERPROFILE%\<folder>` (e.g. `C:\Users\<user>\Documents`)                                       |
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::os_api::{get_public_dir, BrowseLocation};
///
/// // Resolved accordingly to the current OS: XDG on Linux, `~/` or `%USERPROFILE%` folders elsewhere.
/// if let Some(documents) = get_public_dir(BrowseLocation::Documents) {
///     println!("Documents folder: {documents:?}");
/// }
/// ```
#[cfg(not(target_os = "android"))]
pub fn get_public_dir(dir_type: BrowseLocation) -> Option<PathBuf> {
    use std::env;

    match dir_type {
        BrowseLocation::Documents => xdg_or_home_dir("XDG_DOCUMENTS_DIR", "Documents"),
        BrowseLocation::Pictures => xdg_or_home_dir("XDG_PICTURES_DIR", "Pictures"),
        BrowseLocation::Movies => xdg_or_home_dir("XDG_VIDEOS_DIR", "Videos"),
        BrowseLocation::Music => xdg_or_home_dir("XDG_MUSIC_DIR", "Music"),
        BrowseLocation::Downloads => xdg_or_home_dir("XDG_DOWNLOAD_DIR", "Downloads"),
        // Internal / External are normally handled by `BrowseLocation::get_path`,
        // but keep the function behavior coherent if it is called directly.
        BrowseLocation::Internal => env::current_dir().ok(),
        BrowseLocation::External => get_external_path(&None),
    }
}

/// Try an XDG user dir env var first, else fall back to `$HOME/<folder>`.
///
/// Linux implementation, the resolution order is:
/// 1. the `XDG_*_DIR` environment variable if set (e.g. `XDG_DOCUMENTS_DIR`),
/// 2. the `${XDG_CONFIG_HOME:-$HOME/.config}/user-dirs.dirs` file written by
///    `xdg-user-dirs` (where Linux desktops actually store those paths),
/// 3. `$HOME/<fallback_name>` as a last resort.
#[cfg(target_os = "linux")]
fn xdg_or_home_dir(env_var: &str, fallback_name: &str) -> Option<PathBuf> {
    use std::env;

    // 1. The XDG_*_DIR environment variable.
    if let Ok(dir) = env::var(env_var)
        && !dir.is_empty()
    {
        return Some(PathBuf::from(dir));
    }

    let home = env::var("HOME").ok()?;
    let home_path = PathBuf::from(&home);

    // 2. The user-dirs.dirs config file, e.g. `XDG_DOCUMENTS_DIR="$HOME/Documents"`.
    //    XDG_CONFIG_HOME must be absolute to be valid, otherwise fall back to ~/.config.
    let config_dir = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|dir| Path::new(dir).is_absolute())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_path.join(".config"));

    if let Ok(user_dirs) = std::fs::read_to_string(config_dir.join("user-dirs.dirs")) {
        for line in user_dirs.lines() {
            if let Some(value) = user_dirs_value(line, env_var) {
                let path = PathBuf::from(value.replace("$HOME", &home));
                if path.is_absolute() {
                    return Some(path);
                }
            }
        }
    }

    // 3. $HOME/<fallback_name>
    Some(home_path.join(fallback_name))
}

/// Extract the value of `env_var` from a `user-dirs.dirs` line.
///
/// The expected line format is `<env_var>="$HOME/<folder>"` (or an absolute path),
/// comment lines (starting with `#`) are ignored.
///
/// Returns the unquoted value, with `$HOME` still to expand.
#[cfg(target_os = "linux")]
fn user_dirs_value<'a>(line: &'a str, env_var: &str) -> Option<&'a str> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let value = line.strip_prefix(env_var)?.trim_start().strip_prefix('=')?.trim();
    let value = value.trim_matches('"');
    if value.is_empty() { None } else { Some(value) }
}

/// macOS implementation: the user folders always live directly under `$HOME`
/// (macOS doesn't follow the XDG spec), so the `env_var` parameter is ignored.
///
/// The XDG video user dir (`Videos`) is named `Movies` on macOS.
#[cfg(target_os = "macos")]
fn xdg_or_home_dir(_env_var: &str, fallback_name: &str) -> Option<PathBuf> {
    use std::env;

    let folder_name = match fallback_name {
        "Videos" => "Movies",
        otherwise => otherwise,
    };

    let home = env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(folder_name))
}

/// Windows implementation: the user folders always live directly under
/// `%USERPROFILE%`, so the `env_var` XDG parameter is ignored.
///
/// The resolution is `%USERPROFILE%\<fallback_name>` (e.g. `C:\Users\<user>\Documents`),
/// `HOME` is kept as a fallback for MSYS/Cygwin like environments.
#[cfg(target_os = "windows")]
fn xdg_or_home_dir(_env_var: &str, fallback_name: &str) -> Option<PathBuf> {
    use std::env;

    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).ok()?;
    Some(PathBuf::from(home).join(fallback_name))
}

/// Get the current locale of the system. For Android, it uses Java APIs to get the default locale.
#[cfg(target_os = "android")]
pub fn get_locale() -> String {
    use jni::{jni_sig, jni_str, objects::JString};

    let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
    vm.attach_current_thread(|env| -> jni::errors::Result<String> {
        let locale_class = env.find_class(jni_str!("java/util/Locale"))?;
        let default_locale = env
            .call_static_method(&locale_class, jni_str!("getDefault"), jni_sig!("()Ljava/util/Locale;"), &[])?
            .l()?;

        let locale_str =
            env.call_method(&default_locale, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;

        let java_str = unsafe { JString::from_raw(env, locale_str.into_raw() as jni::sys::jstring) };
        let rust_str: String = java_str.to_string();

        Ok(rust_str) // i.e "fr_FR"
    })
    .unwrap_or_else(|e| {
        Log::warn(format!("Failed to get locale from Android: {}", e));
        String::from("en_US")
    })
}

/// Get the current locale of the system. For Linux
#[cfg(not(target_os = "android"))]
pub fn get_locale() -> String {
    let vars = ["LANG", "LANGUAGE", "LC_ALL", "LC_CTYPE"];
    for var in vars.iter() {
        if let Ok(val) = std::env::var(var)
            && !val.is_empty()
        {
            return val;
        }
    }
    String::new()
}

/// Open an asset like a file
#[cfg(target_os = "android")]
pub fn open_asset(sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<File> {
    use std::ffi::CString;

    if sk_info.is_none() {
        Log::err("open_asset, sk_info is None");
        return None;
    }

    let sk_i = sk_info.as_ref().expect("sk_info should be Some").borrow_mut();
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
/// # sk::Sk::shutdown();
/// ```
#[cfg(not(target_os = "android"))]
pub fn open_asset(_sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<File> {
    use std::env;

    let path_assets = env::current_dir().unwrap_or_default().join(get_assets_dir());
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

    let sk_i = sk_info.as_ref().expect("sk_info should be Some").borrow_mut();
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
///
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// let asset_path = "textures/readme.md";
///
/// let buffer = read_asset(&sk_info, asset_path).expect("File readme should be readable");
/// assert!(buffer.starts_with(b"# Images"));
/// # sk::Sk::shutdown();
/// ```
#[cfg(not(target_os = "android"))]
pub fn read_asset(_sk_info: &Option<Rc<RefCell<SkInfo>>>, asset_path: impl AsRef<Path>) -> Option<Vec<u8>> {
    use std::{env, io::Read};

    let path_assets = env::current_dir().unwrap_or_default().join(get_assets_dir());
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
/// # sk::Sk::shutdown();
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

/// Open IME keyboard. Does nothing on Quest
#[cfg(target_os = "android")]
pub fn show_soft_input_ime(sk_info: &Option<Rc<RefCell<SkInfo>>>, show: bool) -> bool {
    if sk_info.is_none() {
        Log::err("show_soft_input_ime, sk_info is None");
        return false;
    }

    let sk_i = sk_info.as_ref().expect("sk_info should be Some").borrow_mut();
    let app = sk_i.get_android_app();
    if show {
        app.show_soft_input(false);
    } else {
        app.hide_soft_input(false);
    }
    true
}
/// Open nothing has we don't have an IME keyboard
#[cfg(not(target_os = "android"))]
pub fn show_soft_input_ime(_sk_info: &Option<Rc<RefCell<SkInfo>>>, _show: bool) -> bool {
    false
}

/// Open the default browser. Adapted from https://github.com/amodm/webbrowser-rs
#[cfg(target_os = "android")]
pub fn launch_browser_android(url: &str) -> bool {
    Log::diag(format!("launch_browser_android: Attempting to open URL: {}", url));
    match start_android_intent(&AndroidIntent::View { uri: url.to_string(), browsable: false }) {
        Ok(()) => true,
        Err(error) => {
            Log::err(format!("launch_browser_android error: {}", error));
            false
        }
    }
}

/// Open nothing has we aren't on Android
#[cfg(not(target_os = "android"))]
pub fn launch_browser_android(_url: &str) -> bool {
    false
}

/// The Android package of the Meta Horizon OS shell which handles the system deep links.
#[cfg(target_os = "android")]
const VR_SHELL_PACKAGE: &str = "com.oculus.vrshell";

/// The Meta Horizon OS shell activity which receives the system deep links: this is the activity
/// used by the `adb` command of Meta's guide (`com.oculus.vrshell/.MainActivity`).
#[cfg(target_os = "android")]
const VR_SHELL_ACTIVITY: &str = "com.oculus.vrshell.MainActivity";

/// `Intent.FLAG_ACTIVITY_NEW_TASK`: start the target activity in its own task.
///
/// Without it, the activity is started inside the task of this app, which is a Full Space task on
/// Android XR, and it then stays hidden behind the app (browser, Play Store, document picker, ...).
#[cfg(target_os = "android")]
const FLAG_ACTIVITY_NEW_TASK: i32 = 0x1000_0000;

/// Whether the Android Java VM and the activity are available: StereoKit only knows them once
/// `Sk::initialize` has run.
#[cfg(target_os = "android")]
fn android_jni_ready() -> bool {
    !BackendAndroid::java_vm().is_null() && !BackendAndroid::activity().is_null()
}

/// A system deep link action, following the
/// [Meta Horizon OS System Deep Linking guide](https://developers.meta.com/vr/documentation/native/ps-system-deep-linking/).
///
/// Meta documents the **Intent** (`systemux://...`) and, for some actions, the **URI** that the VR
/// Shell (`com.oculus.vrshell`) expects. [`system_deep_link`] sends both to the VR Shell on Horizon
/// OS.
///
/// Android XR has no `systemux://` shell, so on such a device [`system_deep_link`] translates the
/// action to the matching standard Android intent:
///
/// | Action | Horizon OS `intent_data` | Horizon OS `uri` | Android XR intent |
/// | --- | --- | --- | --- |
/// | [`SystemAction::Browser`] | `systemux://browser` | the URL | `ACTION_VIEW` + `CATEGORY_BROWSABLE` |
/// | [`SystemAction::Store`] | `systemux://store` | `/item/<app id>` | the Play Store page of the package |
/// | [`SystemAction::Settings`] | `systemux://settings` | see the variant | `android.settings.*` |
/// | [`SystemAction::FileManager`] | `systemux://file-manager` | `/media/`, `/downloads/` | `ACTION_OPEN_DOCUMENT` |
/// | [`SystemAction::BugReport`] | `systemux://bug_report` | none | `ACTION_BUG_REPORT` |
#[derive(Debug, Clone, PartialEq)]
pub enum SystemAction {
    /// Open the browser with a URL.
    ///
    /// Horizon OS: `systemux://browser` with the URL as URI. Android XR: `ACTION_VIEW` with
    /// `CATEGORY_BROWSABLE`, which opens the default browser (usually in a panel, in the Home
    /// Space).
    Browser {
        /// The URL to open.
        url: String,
    },
    /// Open the store.
    ///
    /// Horizon OS: `systemux://store` without URI for the front page, or `/item/<ID>` for a given
    /// app, `ID` being the numeric Meta app ID.
    ///
    /// Android XR: the Google Play Store page of the app, so `app_id` must then be an **Android
    /// package name** (for example `com.example.app`). A numeric Meta app ID can't be opened by the
    /// Play Store, and [`system_deep_link`] returns `false` for it.
    Store {
        /// `None` for the store front page, or the app ID.
        app_id: Option<String>,
    },
    /// Open the settings.
    ///
    /// Horizon OS: `systemux://settings` with one of the URIs documented by Meta:
    /// - `None`: the settings main page,
    /// - `/hands`: the hand tracking settings page,
    /// - `/applications?package=com.X.Y`: the settings page of an installed app.
    ///
    /// The other paths (`/system`, `/privacy`, `/controllers`, `/bluetooth`, `/wifi`, `/device`,
    /// `/guardian`, `/accounts`, `/notifications`, ...) aren't in Meta's current documentation, and
    /// may not open anything on recent Horizon OS versions; they are still forwarded to the VR
    /// Shell as is.
    ///
    /// Android XR: translated to the closest public `android.settings.*` action: `/wifi` →
    /// `WIFI_SETTINGS`, `/bluetooth` and `/controllers` → `BLUETOOTH_SETTINGS`, `/device` →
    /// `DISPLAY_SETTINGS`, `/privacy` → `PRIVACY_SETTINGS`, `/accounts` → `SYNC_SETTINGS`,
    /// `/notifications` → `ALL_APPS_NOTIFICATION_SETTINGS`, `/applications?package=com.X.Y` →
    /// `APPLICATION_DETAILS_SETTINGS`, and everything else to the main settings page (Android XR has
    /// no public action for hand tracking, guardian, ...).
    ///
    /// A value which already is an Android action (`android.settings.`...) is used as is, which is
    /// the escape hatch to reach an Android XR settings page not covered here.
    ///
    /// On Android XR, the pages of the `com.android.settings` app are displayed above a Full Space
    /// app, but `/privacy` (`PRIVACY_SETTINGS`) is redirected by the system to the permission
    /// controller's Safety Center, which stays hidden behind a Full Space app: open it from the
    /// Home Space.
    ///
    /// For `/applications?package=com.X.Y`, use the real package name of the app (see
    /// [`app_package_name`]): a wrong one makes the settings app open nothing at all.
    Settings {
        /// `None` for the main settings page, or a settings URI/action.
        setting: Option<String>,
    },
    /// Open the Files app.
    ///
    /// Horizon OS: `systemux://file-manager` without URI for the "Recents" tab, `/media/` for the
    /// "Media" tab, or `/downloads/` for the "Downloads" tab.
    ///
    /// Android XR: the system document picker (`ACTION_OPEN_DOCUMENT`); the requested tab can't be
    /// selected.
    FileManager {
        /// `None` for the default tab, or a tab path.
        path: Option<String>,
    },
    /// Open the system bug reporter.
    ///
    /// Horizon OS: `systemux://bug_report`. Android XR: `ACTION_BUG_REPORT`, if the device provides
    /// a handler for it.
    BugReport,
}

impl SystemAction {
    /// The `intent_data` and `uri` extras that the Meta Horizon OS shell (`com.oculus.vrshell`)
    /// expects for this action, following the
    /// [System Deep Linking guide](https://developers.meta.com/vr/documentation/native/ps-system-deep-linking/).
    ///
    /// Returns `(intent_data, uri)`, `uri` being empty when the action doesn't use one.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::tools::os_api::SystemAction;
    ///
    /// assert_eq!(
    ///     SystemAction::Browser { url: "https://www.meta.com".to_string() }.horizon_deep_link(),
    ///     ("systemux://browser", "https://www.meta.com".to_string())
    /// );
    /// assert_eq!(
    ///     SystemAction::Store { app_id: Some("1234567890".to_string()) }.horizon_deep_link(),
    ///     ("systemux://store", "/item/1234567890".to_string())
    /// );
    /// ```
    pub fn horizon_deep_link(&self) -> (&'static str, String) {
        match self {
            SystemAction::Browser { url } => ("systemux://browser", url.clone()),
            SystemAction::Store { app_id } => {
                ("systemux://store", app_id.as_ref().map(|app_id| format!("/item/{app_id}")).unwrap_or_default())
            }
            SystemAction::Settings { setting } => {
                ("systemux://settings", setting.as_deref().unwrap_or_default().to_string())
            }
            SystemAction::FileManager { path } => {
                ("systemux://file-manager", path.as_deref().unwrap_or_default().to_string())
            }
            SystemAction::BugReport => ("systemux://bug_report", String::new()),
        }
    }

    /// A short description of this action, for the logs.
    fn describe(&self) -> String {
        match self {
            SystemAction::Browser { url } => format!("opening the browser at {url}"),
            SystemAction::Store { app_id } => match app_id {
                Some(app_id) => format!("opening the store page of {app_id}"),
                None => "opening the store front page".to_string(),
            },
            SystemAction::Settings { setting } => match setting.as_deref().filter(|setting| !setting.is_empty()) {
                Some(setting) => format!("opening the settings page {setting}"),
                None => "opening the settings".to_string(),
            },
            SystemAction::FileManager { path } => match path.as_deref().filter(|path| !path.is_empty()) {
                Some(path) => format!("opening the Files app at {path}"),
                None => "opening the Files app".to_string(),
            },
            SystemAction::BugReport => "opening the bug reporter".to_string(),
        }
    }
}

/// A standard Android intent, used for the platforms which have no Meta VR Shell (Android XR,
/// Android phones, ...).
#[cfg(any(target_os = "android", test))]
#[derive(Debug, Clone, PartialEq)]
enum AndroidIntent {
    /// `Intent(Intent.ACTION_VIEW, Uri.parse(uri))`, with `Intent.CATEGORY_BROWSABLE` if `browsable`.
    View {
        /// The URI to open.
        uri: String,
        /// Whether to add `CATEGORY_BROWSABLE` (restrict the intent to browser-like apps).
        browsable: bool,
    },
    /// `Intent(action)`, with an optional data URI.
    Action {
        /// The intent action (for example `android.settings.SETTINGS`).
        action: String,
        /// The optional data URI (for example `package:com.example.app`).
        data: Option<String>,
    },
    /// `Intent(Intent.ACTION_OPEN_DOCUMENT)` with `CATEGORY_OPENABLE` and a MIME type.
    OpenDocument {
        /// The MIME type of the documents to pick.
        mime: String,
    },
}

#[cfg(any(target_os = "android", test))]
impl AndroidIntent {
    /// A short description of this intent, for the logs.
    #[cfg(target_os = "android")]
    fn describe(&self) -> String {
        match self {
            AndroidIntent::View { uri, .. } => format!("ACTION_VIEW {uri}"),
            AndroidIntent::Action { action, data } => match data {
                Some(data) => format!("{action} ({data})"),
                None => action.clone(),
            },
            AndroidIntent::OpenDocument { mime } => format!("ACTION_OPEN_DOCUMENT ({mime})"),
        }
    }
}

#[cfg(any(target_os = "android", test))]
impl SystemAction {
    /// The standard Android intent which does the same thing as this action, for the devices
    /// without a Meta VR Shell (Android XR, Android phones, ...).
    ///
    /// Returns the reason of the failure when the action has no Android equivalent.
    fn android_intent(&self) -> std::result::Result<AndroidIntent, String> {
        Ok(match self {
            SystemAction::Browser { url } => AndroidIntent::View { uri: url.clone(), browsable: true },
            SystemAction::Store { app_id } => {
                let uri = match app_id.as_deref() {
                    None => "https://play.google.com/store/apps".to_string(),
                    // The Play Store works with Android package names, not with Meta's numeric app
                    // IDs.
                    Some(app_id) if app_id.contains('.') => {
                        format!("https://play.google.com/store/apps/details?id={app_id}")
                    }
                    Some(app_id) => {
                        return Err(format!(
                            "'{app_id}' is not an Android package name: pass the Android package of the app \
                             (like 'com.example.app') to open its Play Store page on Android XR"
                        ));
                    }
                };
                AndroidIntent::View { uri, browsable: false }
            }
            SystemAction::Settings { setting } => {
                let Some(setting) = setting.as_deref().map(str::trim).filter(|setting| !setting.is_empty()) else {
                    return Ok(AndroidIntent::Action { action: "android.settings.SETTINGS".to_string(), data: None });
                };
                // Escape hatch for the settings pages we don't map.
                if setting.starts_with("android.settings.") {
                    return Ok(AndroidIntent::Action { action: setting.to_string(), data: None });
                }
                if let Some(package) = setting.strip_prefix("/applications?package=") {
                    return Ok(AndroidIntent::Action {
                        action: "android.settings.APPLICATION_DETAILS_SETTINGS".to_string(),
                        data: Some(format!("package:{package}")),
                    });
                }
                let action = match setting {
                    "/wifi" => "android.settings.WIFI_SETTINGS",
                    "/bluetooth" | "/controllers" => "android.settings.BLUETOOTH_SETTINGS",
                    "/device" => "android.settings.DISPLAY_SETTINGS",
                    "/privacy" => "android.settings.PRIVACY_SETTINGS",
                    "/accounts" => "android.settings.SYNC_SETTINGS",
                    "/notifications" => "android.settings.ALL_APPS_NOTIFICATION_SETTINGS",
                    // `/hands`, `/guardian`, `/system`, ... have no public Android XR action: the
                    // main settings page is the closest thing we can open.
                    _ => "android.settings.SETTINGS",
                };
                AndroidIntent::Action { action: action.to_string(), data: None }
            }
            // The document picker opens on its last tab; Android has no intent to choose one.
            SystemAction::FileManager { .. } => AndroidIntent::OpenDocument { mime: "*/*".to_string() },
            SystemAction::BugReport => {
                AndroidIntent::Action { action: "android.intent.action.BUG_REPORT".to_string(), data: None }
            }
        })
    }
}

/// Calls `android.net.Uri.parse` and returns the resulting `Uri`.
#[cfg(target_os = "android")]
fn android_uri<'local>(env: &mut jni::Env<'local>, uri: &str) -> jni::errors::Result<jni::objects::JObject<'local>> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JValue},
    };

    let uri_class = env.find_class(jni_str!("android/net/Uri"))?;
    let uri_string = env.new_string(uri)?;
    env.call_static_method(
        &uri_class,
        jni_str!("parse"),
        jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
        &[JValue::Object(&JObject::from(uri_string))],
    )?
    .l()
}

/// The class name and the message of the Java exception which is being thrown, if any, after
/// clearing it: a pending exception makes every other JNI call unreliable.
#[cfg(target_os = "android")]
fn take_java_exception(env: &mut jni::Env<'_>) -> Option<(String, String)> {
    match env.exception_catch() {
        Ok(()) => None,
        Err(jni::errors::Error::CaughtJavaException { name, msg, .. }) => Some((name, msg)),
        Err(error) => Some(("unknown".to_string(), error.to_string())),
    }
}

/// Why a Java call failed.
#[cfg(target_os = "android")]
enum JavaFailure {
    /// The Java exception which was thrown, already caught and cleared: its class name and message.
    Exception {
        /// The fully qualified class name (for example `android.content.ActivityNotFoundException`).
        name: String,
        /// The exception message.
        message: String,
    },
    /// A JNI level failure, without any Java exception.
    Jni(String),
}

#[cfg(target_os = "android")]
impl JavaFailure {
    /// A human readable description, for the logs.
    fn describe(&self) -> String {
        match self {
            JavaFailure::Exception { name, message } => format!("{name}: {message}"),
            JavaFailure::Jni(error) => format!("JNI error: {error}"),
        }
    }
}

/// Calls `Activity.startActivity(intent)` on the activity of the app.
///
/// `jni` post-checks every call and returns `Err(JavaException)` while leaving the exception
/// pending, so it has to be caught here: that's how the class of the exception is known (see
/// [`JavaFailure::Exception`]), and a pending exception must not be left behind (the JVM would
/// rethrow it when the thread is detached).
#[cfg(target_os = "android")]
fn start_activity(env: &mut jni::Env<'_>, intent: &jni::objects::JObject<'_>) -> std::result::Result<(), JavaFailure> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JValue},
    };

    let activity = unsafe { JObject::from_raw(env, BackendAndroid::activity() as _) };
    match env.call_method(
        &activity,
        jni_str!("startActivity"),
        jni_sig!("(Landroid/content/Intent;)V"),
        &[JValue::Object(intent)],
    ) {
        Ok(_) => Ok(()),
        Err(error) => Err(match take_java_exception(env) {
            Some((name, message)) => JavaFailure::Exception { name, message },
            None => JavaFailure::Jni(error.to_string()),
        }),
    }
}

/// Starts `intent` from the Android activity of the app.
///
/// Returns the Java exception name and message when the system refused the intent (no app for this
/// action, ...).
#[cfg(target_os = "android")]
fn start_android_intent(intent: &AndroidIntent) -> std::result::Result<(), String> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JValue},
    };

    if !android_jni_ready() {
        return Err("the Android Java VM isn't available yet (call system_deep_link after Sk::initialize)".to_string());
    }

    let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
    let started = vm.attach_current_thread(|env| -> jni::errors::Result<std::result::Result<(), String>> {
        let intent_class = env.find_class(jni_str!("android/content/Intent"))?;

        let java_intent = match intent {
            AndroidIntent::View { uri, browsable } => {
                let action = env
                    .get_static_field(&intent_class, jni_str!("ACTION_VIEW"), jni_sig!("Ljava/lang/String;"))?
                    .l()?;
                let data = android_uri(env, uri)?;
                let java_intent = env.new_object(
                    &intent_class,
                    jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
                    &[JValue::Object(&action), JValue::Object(&data)],
                )?;
                if *browsable {
                    let category = env
                        .get_static_field(
                            &intent_class,
                            jni_str!("CATEGORY_BROWSABLE"),
                            jni_sig!("Ljava/lang/String;"),
                        )?
                        .l()?;
                    env.call_method(
                        &java_intent,
                        jni_str!("addCategory"),
                        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                        &[JValue::Object(&category)],
                    )?;
                }
                java_intent
            }
            AndroidIntent::Action { action, data } => {
                let data = match data {
                    Some(data) => android_uri(env, data)?,
                    None => JObject::null(),
                };
                let action = env.new_string(action)?;
                env.new_object(
                    &intent_class,
                    jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
                    &[JValue::Object(&JObject::from(action)), JValue::Object(&data)],
                )?
            }
            AndroidIntent::OpenDocument { mime } => {
                let action = env
                    .get_static_field(&intent_class, jni_str!("ACTION_OPEN_DOCUMENT"), jni_sig!("Ljava/lang/String;"))?
                    .l()?;
                let java_intent =
                    env.new_object(&intent_class, jni_sig!("(Ljava/lang/String;)V"), &[JValue::Object(&action)])?;
                let mime = env.new_string(mime)?;
                env.call_method(
                    &java_intent,
                    jni_str!("setType"),
                    jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                    &[JValue::Object(&JObject::from(mime))],
                )?;
                let category = env
                    .get_static_field(&intent_class, jni_str!("CATEGORY_OPENABLE"), jni_sig!("Ljava/lang/String;"))?
                    .l()?;
                env.call_method(
                    &java_intent,
                    jni_str!("addCategory"),
                    jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                    &[JValue::Object(&category)],
                )?;
                java_intent
            }
        };
        // Launch the target in its own task: started inside this app's task (a Full Space task on
        // Android XR), the browser / Play Store / document picker would open behind the app.
        env.call_method(
            &java_intent,
            jni_str!("addFlags"),
            jni_sig!("(I)Landroid/content/Intent;"),
            &[jni::objects::JValue::Int(FLAG_ACTIVITY_NEW_TASK)],
        )?;

        Ok(start_activity(env, &java_intent).map_err(|failure| failure.describe()))
    });

    match started {
        Ok(started) => started,
        Err(error) => Err(format!("JNI error: {error}")),
    }
}

/// Calls `PackageManager.getLaunchIntentForPackage` for the Meta VR Shell, and returns its launch
/// intent (`null` when the VR Shell isn't installed or is hidden by the package visibility rules).
#[cfg(target_os = "android")]
fn vr_shell_launch_intent<'local>(env: &mut jni::Env<'local>) -> jni::errors::Result<jni::objects::JObject<'local>> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JValue},
    };

    let activity = unsafe { JObject::from_raw(env, BackendAndroid::activity() as _) };
    let package_manager = env
        .call_method(&activity, jni_str!("getPackageManager"), jni_sig!("()Landroid/content/pm/PackageManager;"), &[])?
        .l()?;
    let package_name = env.new_string(VR_SHELL_PACKAGE)?;
    env.call_method(
        &package_manager,
        jni_str!("getLaunchIntentForPackage"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&JObject::from(package_name))],
    )?
    .l()
}

/// Sends a Meta Horizon OS system deep link to the VR Shell.
///
/// Returns `Ok(true)` when the intent was handed to the VR Shell, `Ok(false)` when this device has
/// no VR Shell at all (Android XR, Android phone, ...) so the caller can use a standard Android
/// intent instead, and `Err` when the VR Shell exists but couldn't be started.
#[cfg(target_os = "android")]
fn start_vr_shell_deep_link(intent_data: &str, uri: &str) -> std::result::Result<bool, String> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JValue},
    };

    if !android_jni_ready() {
        return Err("the Android Java VM isn't available yet (call system_deep_link after Sk::initialize)".to_string());
    }

    let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
    let started = vm.attach_current_thread(|env| -> jni::errors::Result<std::result::Result<bool, String>> {
        // The documented way: the launch intent of the VR Shell.
        let launch_intent = vr_shell_launch_intent(env)?;

        // With `targetSdkVersion` 30 and up, the package visibility rules hide the VR Shell from
        // `getLaunchIntentForPackage` unless the manifest declares it in a `<queries>` element. The
        // explicit intent of the `adb` command in Meta's guide still works in that case.
        let explicit = launch_intent.is_null();
        let java_intent = if explicit {
            let intent_class = env.find_class(jni_str!("android/content/Intent"))?;
            let action =
                env.get_static_field(&intent_class, jni_str!("ACTION_VIEW"), jni_sig!("Ljava/lang/String;"))?.l()?;
            let data = android_uri(env, intent_data)?;
            let java_intent = env.new_object(
                &intent_class,
                jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
                &[JValue::Object(&action), JValue::Object(&data)],
            )?;
            let package = env.new_string(VR_SHELL_PACKAGE)?;
            let class = env.new_string(VR_SHELL_ACTIVITY)?;
            env.call_method(
                &java_intent,
                jni_str!("setClassName"),
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&JObject::from(package)), JValue::Object(&JObject::from(class))],
            )?;
            java_intent
        } else {
            launch_intent
        };

        // `intent_data` is the action to run (`systemux://...`), `uri` its optional parameter.
        for (key, value) in [("intent_data", intent_data), ("uri", uri)] {
            if value.is_empty() {
                continue;
            }
            let java_key = env.new_string(key)?;
            let java_value = env.new_string(value)?;
            env.call_method(
                &java_intent,
                jni_str!("putExtra"),
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&JObject::from(java_key)), JValue::Object(&JObject::from(java_value))],
            )?;
        }

        Ok(match start_activity(env, &java_intent) {
            Ok(()) => Ok(true),
            // When the VR Shell isn't installed (Android XR), only the explicit intent fails, and it
            // fails with ActivityNotFoundException: this device simply doesn't run Horizon OS.
            Err(JavaFailure::Exception { name, .. }) if explicit && name.ends_with("ActivityNotFoundException") => {
                Ok(false)
            }
            Err(failure) => Err(format!("startActivity({VR_SHELL_PACKAGE}) failed: {}", failure.describe())),
        })
    });

    match started {
        Ok(started) => started,
        Err(error) => Err(format!("JNI error: {error}")),
    }
}

/// Triggers a system deep link: the Meta Horizon OS `systemux://` intent when the VR Shell is
/// available, or the equivalent standard Android intent on Android XR (and on any other Android
/// device).
///
/// On Meta Horizon OS this follows the
/// [System Deep Linking guide](https://developers.meta.com/vr/documentation/native/ps-system-deep-linking/):
/// the `intent_data` and `uri` extras are sent to `com.oculus.vrshell`, through its launch intent
/// or, when the package visibility rules hide it (targetSdk 30+ without a `<queries>` entry for
/// `com.oculus.vrshell`), through the explicit `com.oculus.vrshell/.MainActivity` intent used by
/// the `adb` command of the guide. Adding
///
/// ```xml
/// <queries>
///     <package android:name="com.oculus.vrshell" />
/// </queries>
/// ```
///
/// to your `AndroidManifest.xml` is still recommended on Horizon OS.
///
/// Android XR has no `systemux://` shell: the action is then translated to a standard Android
/// intent (see [`SystemAction`]), and the target app opens the same way as when the user launches
/// it, in its own panel in the Home Space. Embedding it inside your app's space would need the
/// Jetpack XR `ActivityPanelEntity` and the `EMBED_ACTIVITY` capability, which aren't reachable
/// from an OpenXR app.
///
/// See also [`SystemAction::horizon_deep_link`].
///
/// # Return
/// `true` when the intent was handed to the system, `false` when the platform refused it (no
/// handler for the action, numeric Meta store ID on Android XR, not an Android platform, ...).
///
/// # Testing with `adb`
/// On Horizon OS, the same deep link can be triggered without your app (see Meta's guide):
/// ```text
/// adb shell am start -a android.intent.action.VIEW -n com.oculus.vrshell/.MainActivity -d "systemux://settings" -e uri "/hands"
/// ```
/// On Android XR, the translated intent is a plain Android one:
/// ```text
/// adb shell am start -a android.settings.SETTINGS
/// adb shell am start -a android.intent.action.VIEW -d "https://www.meta.com"
/// adb shell am start -a android.intent.action.OPEN_DOCUMENT -t "*/*"
/// ```
///
/// ### Examples
/// ```no_run
/// use stereokit_rust::tools::os_api::{SystemAction, system_deep_link};
///
/// // Open the browser with a URL on Horizon OS, or the default browser on Android XR.
/// system_deep_link(SystemAction::Browser { url: "https://www.meta.com".to_string() });
///
/// // Open the store front page, or the page of an app. On Android XR, `app_id` must be the
/// // Android package name of the app.
/// system_deep_link(SystemAction::Store { app_id: None });
/// system_deep_link(SystemAction::Store { app_id: Some("com.example.app".to_string()) });
///
/// // Open the main settings page, or a specific one.
/// system_deep_link(SystemAction::Settings { setting: None });
/// system_deep_link(SystemAction::Settings { setting: Some("/hands".to_string()) });
/// system_deep_link(SystemAction::Settings { setting: Some("/applications?package=com.example.app".to_string()) });
///
/// // Open the Files app, and the bug reporter.
/// system_deep_link(SystemAction::FileManager { path: None });
/// system_deep_link(SystemAction::BugReport);
/// ```
#[cfg(target_os = "android")]
pub fn system_deep_link(action: SystemAction) -> bool {
    use crate::system::Log;

    Log::info(format!("system_deep_link: {}", action.describe()));

    if is_horizon_os() {
        let (intent_data, uri) = action.horizon_deep_link();
        match start_vr_shell_deep_link(intent_data, &uri) {
            Ok(true) => {
                Log::info(format!("system_deep_link: sent '{intent_data}' (uri: '{uri}') to the Horizon OS VR Shell"));
                return true;
            }
            Ok(false) => Log::diag(format!(
                "system_deep_link: {VR_SHELL_PACKAGE} not available, using a standard Android intent"
            )),
            Err(error) => Log::warn(format!(
                "system_deep_link: Horizon OS VR Shell failed, using a standard Android intent: {error}"
            )),
        }
    }

    match action.android_intent() {
        Ok(intent) => match start_android_intent(&intent) {
            Ok(()) => {
                Log::info(format!("system_deep_link: started {}", intent.describe()));
                true
            }
            Err(error) => {
                Log::warn(format!("system_deep_link: {} failed: {error}", intent.describe()));
                false
            }
        },
        Err(message) => {
            Log::warn(format!("system_deep_link: {message}"));
            false
        }
    }
}

/// Triggers a system deep link. See the Android version of this function for the details.
///
/// This is a no-op returning `false` on the platforms which aren't Android.
#[cfg(not(target_os = "android"))]
pub fn system_deep_link(action: SystemAction) -> bool {
    use crate::system::Log;
    Log::warn(format!("system_deep_link: not supported on this platform, ignoring '{}'", action.describe()));
    false
}

/// The Android package name of the app, or `None` on the platforms which aren't Android and before
/// `Sk::initialize`.
///
/// It's what goes in the `/applications?package=...` settings URI, which is how
/// [`SystemAction::Settings`] opens the settings page of the app itself on both Horizon OS and
/// Android XR. Hard coding that package name in the code breaks as soon as the app is renamed.
///
/// ### Examples
/// ```no_run
/// use stereokit_rust::tools::os_api::{SystemAction, app_package_name, system_deep_link};
///
/// if let Some(package) = app_package_name() {
///     system_deep_link(SystemAction::Settings { setting: Some(format!("/applications?package={package}")) });
/// }
/// ```
#[cfg(target_os = "android")]
pub fn app_package_name() -> Option<String> {
    use jni::{
        jni_sig, jni_str,
        objects::{JObject, JString},
    };

    if !android_jni_ready() {
        return None;
    }
    let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
    vm.attach_current_thread(|env| -> jni::errors::Result<String> {
        let activity = unsafe { JObject::from_raw(env, BackendAndroid::activity() as _) };
        let package =
            env.call_method(&activity, jni_str!("getPackageName"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let package = unsafe { JString::from_raw(env, package.into_raw() as jni::sys::jstring) };
        package.try_to_string(env)
    })
    .ok()
}

/// The app package name. Always `None` on the platforms which aren't Android.
///
/// see also [`app_package_name`]
#[cfg(not(target_os = "android"))]
pub fn app_package_name() -> Option<String> {
    None
}

/// Whether the app runs on Meta Horizon OS, the only platform where the `systemux://` system deep
/// links are available.
///
/// The check looks for the Meta VR Shell package (which handles the deep links) and, when the
/// package visibility rules hide it, for the Meta manufacturer/brand in `android.os.Build`. The
/// answer is cached after the first call, since it can't change while the app runs.
///
/// It returns `false` on Android XR, on plain Android devices, on the platforms which aren't
/// Android, and before `Sk::initialize` (the Java VM isn't known yet).
///
/// see also [`system_deep_link`]
///
/// ### Examples
/// ```no_run
/// use stereokit_rust::tools::os_api::is_horizon_os;
///
/// if is_horizon_os() {
///     println!("Meta Horizon OS: the systemux:// links are available");
/// } else {
///     println!("Android XR or another platform: system_deep_link uses standard Android intents");
/// }
/// ```
#[cfg(target_os = "android")]
pub fn is_horizon_os() -> bool {
    use std::sync::OnceLock;

    static IS_HORIZON_OS: OnceLock<bool> = OnceLock::new();
    if let Some(is_horizon_os) = IS_HORIZON_OS.get() {
        return *is_horizon_os;
    }
    match probe_horizon_os() {
        Some(is_horizon_os) => {
            let _ = IS_HORIZON_OS.set(is_horizon_os);
            is_horizon_os
        }
        // The Java VM isn't available yet, don't cache a wrong answer.
        None => false,
    }
}

/// Whether the app runs on Meta Horizon OS. Always `false` on the platforms which aren't Android.
///
/// see also [`is_horizon_os`]
#[cfg(not(target_os = "android"))]
pub fn is_horizon_os() -> bool {
    false
}

/// The platform detection behind [`is_horizon_os`]: `None` when the Java VM isn't available yet.
#[cfg(target_os = "android")]
fn probe_horizon_os() -> Option<bool> {
    if !android_jni_ready() {
        return None;
    }

    // The VR Shell is what handles the `systemux://` deep links: its presence is the best clue.
    let vr_shell_found = {
        let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
        vm.attach_current_thread(|env| -> jni::errors::Result<bool> {
            vr_shell_launch_intent(env).map(|launch_intent| !launch_intent.is_null())
        })
        .ok()
    };
    if vr_shell_found == Some(true) {
        return Some(true);
    }

    // The package visibility rules (targetSdk 30+ without a `<queries>` entry) can hide the VR
    // Shell, so fall back to the `android.os.Build` fields of the Meta devices.
    let meta_build = ["MANUFACTURER", "BRAND"].iter().any(|field| {
        android_build_field(field).is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("oculus") || value == "meta"
        })
    });
    Some(meta_build)
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
/// # sk::Sk::shutdown();
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

/// Reads a static `String` field of the Java class `android.os.Build` (for example `HARDWARE` or `DEVICE`), and returns
/// its value, or `None` if the field is null, unreadable or if the Java VM isn't available yet.
///
/// `android.os.Build` mirrors the system properties (`HARDWARE` is `ro.hardware`, `DEVICE` is `ro.product.device`, ...)
/// but it's a public Java API, so there's no need for the `libc` crate, an FFI declaration of `__system_property_get` or
/// a `getprop` process. The Java VM is only known after `Sk::initialize`, so this returns `None` before that.
///
/// This is used by [`crate::util::Device::is_emulator`] to detect the Android emulator.
#[cfg(target_os = "android")]
pub fn android_build_field(field: &str) -> Option<String> {
    use jni::{jni_sig, jni_str, objects::JString, strings::JNIString};

    if BackendAndroid::java_vm().is_null() {
        return None;
    }
    let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
    vm.attach_current_thread(|env| -> jni::errors::Result<Option<String>> {
        let build_class = env.find_class(jni_str!("android/os/Build"))?;
        let value = env.get_static_field(&build_class, JNIString::new(field), jni_sig!("Ljava/lang/String;"))?;
        // A pending Java exception (unknown field, wrong signature, ...) makes the following JNI calls unreliable.
        if env.exception_check() {
            env.exception_clear();
            return Ok(None);
        }
        let value = value.l()?;
        if value.is_null() {
            return Ok(None);
        }
        let value = unsafe { JString::from_raw(env, value.into_raw() as jni::sys::jstring) };
        Ok(Some(value.try_to_string(env)?))
    })
    .unwrap_or_else(|error| {
        Log::err(format!("android_build_field({}) failed: {}", field, error));
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizon_deep_links_follow_meta_documentation() {
        assert_eq!(
            SystemAction::Browser { url: "https://www.meta.com".to_string() }.horizon_deep_link(),
            ("systemux://browser", "https://www.meta.com".to_string())
        );
        assert_eq!(SystemAction::Store { app_id: None }.horizon_deep_link(), ("systemux://store", String::new()));
        assert_eq!(
            SystemAction::Store { app_id: Some("1234567890".to_string()) }.horizon_deep_link(),
            ("systemux://store", "/item/1234567890".to_string())
        );
        assert_eq!(
            SystemAction::Settings { setting: Some("/hands".to_string()) }.horizon_deep_link(),
            ("systemux://settings", "/hands".to_string())
        );
        assert_eq!(
            SystemAction::FileManager { path: Some("/downloads/".to_string()) }.horizon_deep_link(),
            ("systemux://file-manager", "/downloads/".to_string())
        );
        assert_eq!(SystemAction::BugReport.horizon_deep_link(), ("systemux://bug_report", String::new()));
    }

    #[test]
    fn android_intents_follow_the_android_xr_mapping() {
        assert_eq!(
            SystemAction::Browser { url: "https://stereokit.net".to_string() }.android_intent().unwrap(),
            AndroidIntent::View { uri: "https://stereokit.net".to_string(), browsable: true }
        );
        assert_eq!(
            SystemAction::Store { app_id: None }.android_intent().unwrap(),
            AndroidIntent::View { uri: "https://play.google.com/store/apps".to_string(), browsable: false }
        );
        assert_eq!(
            SystemAction::Store { app_id: Some("com.example.app".to_string()) }.android_intent().unwrap(),
            AndroidIntent::View {
                uri: "https://play.google.com/store/apps/details?id=com.example.app".to_string(),
                browsable: false
            }
        );
        // Meta's numeric app IDs have no meaning for the Play Store.
        assert!(SystemAction::Store { app_id: Some("1234567890".to_string()) }.android_intent().is_err());

        let settings =
            |setting: &str| SystemAction::Settings { setting: Some(setting.to_string()) }.android_intent().unwrap();
        assert_eq!(
            SystemAction::Settings { setting: None }.android_intent().unwrap(),
            AndroidIntent::Action { action: "android.settings.SETTINGS".to_string(), data: None }
        );
        assert_eq!(
            settings("/wifi"),
            AndroidIntent::Action { action: "android.settings.WIFI_SETTINGS".to_string(), data: None }
        );
        assert_eq!(
            settings("/controllers"),
            AndroidIntent::Action { action: "android.settings.BLUETOOTH_SETTINGS".to_string(), data: None }
        );
        assert_eq!(
            settings("/device"),
            AndroidIntent::Action { action: "android.settings.DISPLAY_SETTINGS".to_string(), data: None }
        );
        // No public Android XR action for hand tracking: the main settings page is the fallback.
        assert_eq!(
            settings("/hands"),
            AndroidIntent::Action { action: "android.settings.SETTINGS".to_string(), data: None }
        );
        assert_eq!(
            settings("/notifications"),
            AndroidIntent::Action { action: "android.settings.ALL_APPS_NOTIFICATION_SETTINGS".to_string(), data: None }
        );
        assert_eq!(
            settings("/applications?package=com.example.app"),
            AndroidIntent::Action {
                action: "android.settings.APPLICATION_DETAILS_SETTINGS".to_string(),
                data: Some("package:com.example.app".to_string())
            }
        );
        assert_eq!(
            settings("android.settings.INPUT_METHOD_SETTINGS"),
            AndroidIntent::Action { action: "android.settings.INPUT_METHOD_SETTINGS".to_string(), data: None }
        );

        assert_eq!(
            SystemAction::FileManager { path: Some("/media/".to_string()) }.android_intent().unwrap(),
            AndroidIntent::OpenDocument { mime: "*/*".to_string() }
        );
        assert_eq!(
            SystemAction::BugReport.android_intent().unwrap(),
            AndroidIntent::Action { action: "android.intent.action.BUG_REPORT".to_string(), data: None }
        );
    }

    #[test]
    fn public_dir_uses_the_per_user_alias_for_emulated_storage() {
        // `StorageVolume.getDirectory()` returns the physical emulated path, which always belongs to
        // the first user: it must be remapped to the per-user alias, whatever the user id.
        assert_eq!(per_user_public_root("/storage/emulated/0"), PathBuf::from(PER_USER_PRIMARY_STORAGE_ROOT));
        assert_eq!(per_user_public_root("/storage/emulated/10"), PathBuf::from(PER_USER_PRIMARY_STORAGE_ROOT));

        // A physical primary volume (adopted SD card) has no per-user alias: it is kept as is.
        assert_eq!(per_user_public_root("/storage/1A2B-3C4D"), PathBuf::from("/storage/1A2B-3C4D"));
        // Neither is an arbitrary mount point.
        assert_eq!(per_user_public_root("/mnt/media_rw/1234"), PathBuf::from("/mnt/media_rw/1234"));
    }
}
