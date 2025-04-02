use std::{
    ffi::OsStr,
    fs::{self, File, create_dir},
    io::{self, BufRead, Error},
    path::{Path, PathBuf},
    process::{Command, exit},
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
/// assert!(skshaderc_path.exists());
///
/// let skshaderc_exe_path = get_skshaderc(bin_dir, true);
/// assert!(skshaderc_exe_path.exists());
/// assert!(skshaderc_exe_path.ends_with("skshaderc.exe"));
/// ```
pub fn get_skshaderc(bin_dir: PathBuf, with_wine: bool) -> PathBuf {
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

    let mut skshaderc = bin_dir.clone();
    skshaderc.push(r"StereoKit");
    skshaderc.push(r"tools");
    skshaderc.push(r"skshaderc");
    skshaderc.push(exe_type);
    if cfg!(windows) || with_wine {
        skshaderc.push("skshaderc.exe");
    } else {
        skshaderc.push("skshaderc");
    }
    skshaderc
}

/// Compile hsls file to sks. See config.toml to change the default values.
/// * `project_dir` - The directory of the project. By default it's  the current directory where `shaderc_src` directory
///   is.
/// * `target_dir` - The directory where the sks files will be generated. By default it's the `assets/shaders/`
///   directory.
/// * `options` - The options to pass to skshaderc except -i and -o  that are `project_dir` and `target_dir`.
/// * `with_wine` - If true, use wine to run `skshaderc.exe` on linux.
pub fn compile_hlsl(
    project_dir: PathBuf,
    target_dir: Option<PathBuf>,
    options: &[&str],
    with_wine: bool,
) -> Result<bool, io::Error> {
    //we get the dir from StereoKit-rust (not from here)
    let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let skshaderc = get_skshaderc(bin_dir.clone(), with_wine);

    let mut shaders_source_path = project_dir.clone();

    shaders_source_path.push(get_shaders_source_dir());

    if !shaders_source_path.exists() || !shaders_source_path.is_dir() {
        println!(
            "Current directory do not see {:?} directory. The value may be change in config.toml [env]",
            shaders_source_path
        );
        println!("Abort!");
        exit(1);
    }

    let shaders_path = match target_dir {
        Some(path) => String::from(path.to_str().expect("shader_path can't be a &str!")) + "/",
        None => {
            let mut shaders_path = project_dir.clone();
            shaders_path.push(get_assets_dir());
            if !shaders_path.exists() || !shaders_path.is_dir() {
                return Err(Error::new(
                    io::ErrorKind::Other,
                    format!("Current directory do not see {:?} directory", shaders_path),
                ));
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

    println!("skshaderc executable used :  {:?}", &skshaderc);
    println!("Shaders sources are here : {:?}", &shaders_source_path);
    println!("Shaders compiled there : {:?}", &shaders_path);

    let excluded_extensions = [OsStr::new("hlsli"), OsStr::new("sks"), OsStr::new("txt"), OsStr::new("md")];
    if let Ok(entries) = shaders_source_path.read_dir() {
        for entry in entries {
            let file = entry?.path();
            println!("Compiling file : {:?}", &file);
            if file.is_file() {
                if let Some(extension) = file.extension() {
                    if !excluded_extensions.contains(&extension) {
                        let mut cmd = if with_wine {
                            let mut c = Command::new("wine");
                            c.arg(skshaderc.clone());
                            c
                        } else {
                            Command::new(OsStr::new(skshaderc.to_str().unwrap_or("NOPE")))
                        };
                        cmd.arg("-f").arg("-e").arg("-i").arg(&shaders_include).arg("-o").arg(&shaders_path);
                        for arg in options {
                            cmd.arg(arg);
                        }
                        let output = cmd.arg(file).output().expect("failed to run shader compiler");
                        let out = String::from_utf8(output.clone().stdout).unwrap_or(format!("{:#?}", output));
                        if !out.is_empty() {
                            println!("{}", out)
                        }
                        let err = String::from_utf8(output.clone().stderr).unwrap_or(format!("{:#?}", output));
                        if !err.is_empty() {
                            println!("{}", err)
                        }
                    }
                }
            }
        }
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
                return Ok(line.split("=").last().unwrap().trim().replace("\"", ""));
            }
        } else if line.contains("[package]") {
            in_package = true;
        }
    }
    if in_package {
        Err(Error::new(io::ErrorKind::Other, "Cargo.toml do not have a [package]/name field"))
    } else {
        Err(Error::new(io::ErrorKind::Other, "Cargo.toml do not have a [package] section"))
    }
}
