use crate::sk::SkInfo;
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
    use crate::system::Log;
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

/// Open an asset like a file
#[cfg(target_os = "android")]
pub fn open_asset(sk_info: Rc<RefCell<SkInfo>>, asset_path: impl AsRef<Path>) -> Option<File> {
    use crate::system::Log;
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
