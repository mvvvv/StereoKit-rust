use std::{
    env,
    ffi::OsStr,
    fs::{self, File, create_dir},
    io::{self, BufRead, Error},
    path::{Path, PathBuf},
    process::Command,
};

use crate::tools::os_api::{get_assets_dir, get_shaders_sks_dir, get_shaders_source_dir};

/// Reaching the skshaderc of this platform.
/// * `bin_dir` - The directory of the binaries.
/// * `with_wine` - Whether to use wine to run skshaderc.exe on linux.
///
/// Returns the path to the skshaderc executable.
///
/// # Examples
/// ```
/// use std::path::PathBuf;
/// use stereokit_rust::tools::build_tools::get_skshaderc;
/// let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
/// let skshaderc_path = get_skshaderc(bin_dir.clone(), false);
/// assert!(skshaderc_path.is_ok());
///
/// let skshaderc_exe_path = get_skshaderc(bin_dir, true);
/// assert!(skshaderc_exe_path.is_ok());
/// assert!(skshaderc_exe_path.unwrap_or_default().ends_with("skshaderc.exe"));
/// ```
pub fn get_skshaderc(bin_dir: PathBuf, with_wine: bool) -> Result<PathBuf, io::Error> {
    let mut target_root = bin_dir.clone();
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or("target".into());
    target_root.push(target_dir);

    if !target_root.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} not found. Please run 'cargo build' first.", target_root.display()),
        ));
    }

    let mut tools_dir = target_root.clone();
    tools_dir.push("tools");

    if !with_wine && (cfg!(target_os = "linux") || cfg!(target_os = "macos")) {
        let mut flat_unix = tools_dir.clone();
        flat_unix.push("skshaderc");
        if flat_unix.exists() {
            return Ok(flat_unix);
        }
    }

    if cfg!(windows) || with_wine {
        let mut flat_win = tools_dir.clone();
        flat_win.push("skshaderc.exe");
        if flat_win.exists() {
            return Ok(flat_win);
        }
    }

    // If not found in flat structure, we recall the exe_type structure from sk_gpu to help resolve the problem.
    let target_os = if with_wine {
        "win32"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        ""
    };
    let target_arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        ""
    };
    let exe_type = target_os.to_string() + "_" + target_arch;

    tools_dir.push(exe_type);
    if cfg!(windows) || with_wine {
        tools_dir.push("skshaderc.exe");
    } else {
        tools_dir.push("skshaderc");
    }
    Ok(tools_dir)
}

/// Compile hsls file to sks. Use variables `SK_RUST_SHADERS_SOURCE_DIR`  `SK_RUST_ASSETS_DIR` and `SK_RUST_SHADERS_SKS_DIR`
/// to change the default values.
/// * `project_dir` - The directory of the project. By default it's  the current directory where `shaderc_src` directory
///   is.
/// * `target_dir` - The directory where the sks files will be generated. By default it's the `assets/shaders/`
///   directory.
/// * `options` - The options to pass to skshaderc except -i and -o  that are `project_dir` and `target_dir`.
/// * `with_wine` - If true, use wine to run `skshaderc.exe` on linux.
///
/// Returns `Ok(true)` if the compilation was successful, `Ok(false)` if there was no shaders_src directory and `Err` if
/// there was an error.
pub fn compile_hlsl(
    project_dir: PathBuf,
    target_dir: Option<PathBuf>,
    options: &[&str],
    with_wine: bool,
) -> Result<bool, io::Error> {
    //we get the dir from StereoKit-rust (not from here)
    let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let skshaderc = get_skshaderc(project_dir.clone(), with_wine)?;

    let mut shaders_source_path = project_dir.clone();

    shaders_source_path.push(get_shaders_source_dir());

    if !shaders_source_path.exists() || !shaders_source_path.is_dir() {
        println!(
            "No shaders to compile. Current directory does not see {shaders_source_path:?} directory. \n---The name of the directory may be change with SK_RUST_SHADERS_SOURCE_DIR"
        );
        return Ok(false);
    }

    let shaders_path = match target_dir {
        Some(path) => String::from(path.to_str().expect("shader_path can't be a &str!")) + "/",
        None => {
            let mut shaders_path = project_dir.clone();
            shaders_path.push(get_assets_dir());
            if !shaders_path.exists() || !shaders_path.is_dir() {
                return Err(Error::other(format!("Current directory do not see {shaders_path:?} directory")));
            }

            shaders_path.push(get_shaders_sks_dir());
            if !shaders_path.exists() || !shaders_path.is_dir() {
                create_dir(&shaders_path)?
            }
            String::from(shaders_path.to_str().expect("shader_path can't be a &str!")) + "/"
        }
    };

    let mut shaders_include = bin_dir.clone();
    shaders_include.push("StereoKit");
    shaders_include.push("tools");
    shaders_include.push("include");

    println!("skshaderc executable used :  {:?}", skshaderc);
    println!("Shaders sources are here : {:?}", shaders_source_path);
    println!("Shaders compiled there : {:?}", shaders_path);

    let excluded_extensions = [OsStr::new("hlsli"), OsStr::new("sks"), OsStr::new("txt"), OsStr::new("md")];
    let mut failed_shaders: Vec<PathBuf> = vec![];
    let mut to_compile: Vec<(PathBuf, String)> = vec![];

    if let Ok(entries) = shaders_source_path.read_dir() {
        for entry in entries {
            let entry_path = entry?.path();
            if entry_path.is_dir() {
                let dir_name = entry_path.file_name().ok_or_else(|| Error::other("subdirectory has no name"))?;
                let sub_out = PathBuf::from(&shaders_path).join(dir_name);
                if !sub_out.exists() {
                    create_dir(&sub_out)?;
                }
                let sub_out_str = String::from(sub_out.to_str().expect("sub_shaders_path can't be a &str!")) + "/";
                if let Ok(sub_entries) = entry_path.read_dir() {
                    for sub_entry in sub_entries {
                        let file = sub_entry?.path();
                        if file.is_file()
                            && let Some(extension) = file.extension()
                            && !excluded_extensions.contains(&extension)
                        {
                            to_compile.push((file, sub_out_str.clone()));
                        }
                    }
                }
            } else if entry_path.is_file()
                && let Some(extension) = entry_path.extension()
                && !excluded_extensions.contains(&extension)
            {
                to_compile.push((entry_path, shaders_path.clone()));
            }
        }
    }

    for (file, out_path) in &to_compile {
        println!("Compiling file : {:?}", file);
        let mut cmd = if with_wine {
            let mut c = Command::new("wine");
            c.arg(skshaderc.clone());
            c
        } else {
            Command::new(OsStr::new(skshaderc.to_str().unwrap_or("NOPE")))
        };
        cmd.arg("-f").arg("-e").arg("-i").arg(&shaders_include).arg("-o").arg(out_path);
        for arg in options {
            cmd.arg(arg);
        }
        let output = cmd.arg(file).output().expect("failed to run shader compiler");
        let out = String::from_utf8(output.clone().stdout).unwrap_or(format!("{output:#?}"));
        if !out.is_empty() {
            println!("{out}")
        }
        let err = String::from_utf8(output.clone().stderr).unwrap_or(format!("{output:#?}"));
        if !err.is_empty() {
            println!("{err}")
        }
        if !output.status.success() {
            failed_shaders.push(file.clone());
        }
    }
    if !failed_shaders.is_empty() {
        println!("\x1b[1;31m---Shader compilation failed for {} file(s):", failed_shaders.len());
        for shader in &failed_shaders {
            println!("  - {:?}", shader);
        }
        print!("\x1b[0m");
    }
    Ok(true)
}

/// Recursive fn to copy all the content of a directory to another one.
/// * `src` - The source directory.
/// * `dst` - The destination directory.
pub fn copy_tree(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    if let Err(_err) = fs::create_dir(&dst) {}
    for entry in fs::read_dir(src)?.flatten() {
        let path_type = entry.file_type()?;
        if path_type.is_dir() {
            copy_tree(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Reading Cargo.toml file of the current dir, looking for a \[package\]/name field and returning its value.
///
/// Returns the name of the package as a String or an Error.
/// ### Examples
/// ```
/// use stereokit_rust::tools::build_tools::get_cargo_name;
/// // Create a dummy Cargo.toml file for testing
/// let name = get_cargo_name().expect("name should be found");
/// assert_eq!(name, "stereokit-rust");
/// ```
pub fn get_cargo_name() -> Result<String, Error> {
    // File Cargo.toml must exist in the current path
    let lines = {
        let file = File::open("./Cargo.toml")?;
        io::BufReader::new(file).lines()
    };
    let mut in_package = false;
    // Consumes the iterator, returns an (Optional) String
    for line in lines.map_while(Result::ok) {
        let line = line.trim();
        if in_package {
            if line.starts_with("name=") || line.starts_with("name") {
                return Ok(line.split("=").last().unwrap_or_default().trim().replace("\"", ""));
            }
        } else if line.contains("[package]") {
            in_package = true;
        }
    }
    if in_package {
        Err(Error::other("Cargo.toml do not have a [package]/name field"))
    } else {
        Err(Error::other("Cargo.toml do not have a [package] section"))
    }
}
