use std::{
    env, fs,
    path::{Path, PathBuf},
};

const SK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Template for `src/bin/main_<crate_name>.rs` — shared with the crate-level documentation.
const MAIN_BIN_TEMPLATE: &str = include_str!("../templates/main_bin.rs");

/// Template for `src/lib.rs` — shared with the crate-level documentation.
const LIB_RS_TEMPLATE: &str = include_str!("../templates/lib_rs.rs");

// Framework templates (default)
const MAIN_BIN_FRAMEWORK_TEMPLATE: &str = include_str!("../templates/main_bin_framework.rs");
const LIB_RS_FRAMEWORK_TEMPLATE: &str = include_str!("../templates/lib_rs_framework.rs");
const C_STEPPER_FRAMEWORK_TEMPLATE: &str = include_str!("../templates/c_stepper_framework.rs");

// Gradle templates
const GRADLE_BUILD: &str = include_str!("../templates/gradle/build.gradle");
const GRADLE_SETTINGS: &str = include_str!("../templates/gradle/settings.gradle");
const GRADLE_PROPERTIES: &str = include_str!("../templates/gradle/gradle.properties");
const GRADLE_APP_BUILD: &str = include_str!("../templates/gradle/app_build.gradle");
const GRADLE_MANIFEST: &str = include_str!("../templates/gradle/AndroidManifest.xml");
const GRADLE_MAIN_ACTIVITY: &str = include_str!("../templates/gradle/MainActivity.java");
const GRADLE_LOGCAT_CMD: &str = include_str!("../templates/gradle/logcat.cmd");

// Gradle wrapper files
const GRADLEW: &str = include_str!("../templates/gradle/gradlew");
const GRADLEW_BAT: &str = include_str!("../templates/gradle/gradlew.bat");
const GRADLE_WRAPPER_JAR: &[u8] = include_bytes!("../templates/gradle/gradle/wrapper/gradle-wrapper.jar");
const GRADLE_WRAPPER_PROPS: &str = include_str!("../templates/gradle/gradle/wrapper/gradle-wrapper.properties");
const GRADLE_LIBS_VERSIONS: &str = include_str!("../templates/gradle/gradle/libs.versions.toml");
const GRADLE_WRAPPER_ACTION: &str = include_str!("../templates/gradle/gradle/wrapper/action.yml");

// Asset templates
const ASSET_VR_SPLASH: &[u8] = include_bytes!("../templates/assets/vr_splash.png");

// Res icon templates
const RES_ICON_LDPI: &[u8] = include_bytes!("../templates/res/mipmap-ldpi/app_icon.png");
const RES_ICON_MDPI: &[u8] = include_bytes!("../templates/res/mipmap-mdpi/app_icon.png");
const RES_ICON_HDPI: &[u8] = include_bytes!("../templates/res/mipmap-hdpi/app_icon.png");
const RES_ICON_XHDPI: &[u8] = include_bytes!("../templates/res/mipmap-xhdpi/app_icon.png");
const RES_ICON_XXHDPI: &[u8] = include_bytes!("../templates/res/mipmap-xxhdpi/app_icon.png");
const RES_ICON_XXXHDPI: &[u8] = include_bytes!("../templates/res/mipmap-xxxhdpi/app_icon.png");

pub const USAGE: &str = r#"Create a new StereoKit-rust project.

Usage : cargo new_sk_rs_project [Options] <project_name>
    
    Options:
        --basic        : Use basic templates (without the framework/stepper pattern).
        --no-android   : Don't include Android support code.
        --with-gradle  : Add Gradle files for building an Android APK with cargo-ndk.
        -h|--help      : Display help

    Creates a new Rust library project configured for StereoKit-rust
    with the following structure:
        <project_name>/
            Cargo.toml
            config.toml
            src/
                lib.rs
                bin/
                    main_<project_name>.rs
            assets/
                shaders/
                textures/
                sounds/
                fonts/
"#;

fn show_help() {
    println!("{USAGE}");
}

fn main() {
    let mut project_name = String::new();
    let mut with_android = true;
    let mut with_gradle = false;
    let mut basic = false;

    let args = env::args().skip(1);

    for arg in args {
        match &arg[..] {
            "new_sk_rs_project" => {}
            "--no-android" => {
                with_android = false;
            }
            "--with-gradle" => {
                with_gradle = true;
            }
            "--basic" => {
                basic = true;
            }
            arg if arg == "-h" || arg == "--help" || arg == "--explain" => {
                show_help();
                return;
            }
            _ => {
                if arg.starts_with('-') {
                    println!("Unknown argument {arg}");
                    panic!("{}", USAGE);
                } else if project_name.is_empty() {
                    // Validate project name: alphanumeric, hyphens, underscores, and dots (for package id)
                    if arg.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') && !arg.is_empty() {
                        project_name = arg;
                    } else {
                        println!("Invalid project name: {arg}");
                        println!(
                            "Project name must only contain alphanumeric characters, hyphens, underscores or dots."
                        );
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("Unknown positional argument {arg}");
                    panic!("{}", USAGE);
                }
            }
        }
    }

    if project_name.is_empty() {
        println!("You must specify a project name.");
        panic!("{}", USAGE);
    }

    // If project_name contains dots (e.g. "com.mycompany.my_app"), extract the last segment
    let full_name = project_name.clone();
    if project_name.contains('.') {
        project_name = project_name.rsplit('.').next().unwrap().to_string();
    }

    let project_path = PathBuf::from(&project_name);
    if project_path.exists() {
        panic!("Directory '{}' already exists!", project_name);
    }

    if with_gradle && !with_android {
        println!("--with-gradle requires Android support. Remove --no-android or remove --with-gradle.");
        panic!("{}", USAGE);
    }

    // The crate/module name uses underscores (Rust convention)
    let crate_name = project_name.replace('-', "_");

    println!("Creating StereoKit-rust project: {project_name}");

    // Create directory structure
    let dirs = ["src", "src/bin", "assets/shaders", "assets/textures", "assets/sounds", "assets/fonts"];

    for dir in &dirs {
        let dir_path = project_path.join(dir);
        fs::create_dir_all(&dir_path)
            .unwrap_or_else(|e| panic!("Failed to create directory {}: {e}", dir_path.display()));
    }

    // Copy asset files
    write_bytes(&project_path, "assets/vr_splash.png", ASSET_VR_SPLASH);

    // Generate and write files
    write_cargo_toml(&project_path, &project_name, &crate_name, with_android);
    write_config_toml(&project_path);
    write_lib_rs(&project_path, with_android, basic);
    write_main_rs(&project_path, &crate_name, basic);
    if !basic {
        write_file(&project_path, "src/c_stepper.rs", C_STEPPER_FRAMEWORK_TEMPLATE);
    }

    if with_gradle {
        write_gradle_files(&project_path, &full_name, &crate_name);
    }

    println!();
    println!("Project '{project_name}' created successfully!");
    println!();
    println!("To get started:");
    println!("  cd {project_name}");
    println!("  cargo run --bin main_{crate_name}");
    if with_gradle {
        println!();
        println!("To build and run on an Android headset:");
        if cfg!(target_os = "windows") {
            println!("  .\\gradlew run && .\\logcat.cmd");
        } else {
            println!("  ./gradlew run && sh logcat.cmd");
        }
    }
    println!();
    println!("See https://stereokit.net/ and the stereokit-rust documentation for more information.");
}

fn write_cargo_toml(project_path: &Path, project_name: &str, crate_name: &str, with_android: bool) {
    let android_section = if with_android {
        format!(
            r#"
[target.'cfg(target_os = "android")'.dependencies]
stereokit-rust = {{ version = "{SK_VERSION}", features = ["build-dynamic-openxr"] }}
log = "0.4"
android_logger = "0.15"
android-activity = {{ version = "0.6", features = ["native-activity"] }}
ndk = "0.9.0"
"#
        )
    } else {
        String::new()
    };

    let content = format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["lib", "cdylib"]

[[bin]]
name = "main_{crate_name}"

[dependencies]
stereokit-rust = "{SK_VERSION}"
{android_section}"#
    );

    let path = project_path.join("Cargo.toml");
    fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

fn write_config_toml(project_path: &Path) {
    let content = r#"[env]
# Set ENV_VAR_NAME=value for any process run by Cargo
SK_RUST_ASSETS_DIR = "assets"
SK_RUST_SHADERS_SOURCE_DIR = "shaders_src"
SK_RUST_SHADERS_SKS_DIR = "shaders"
"#;

    let path = project_path.join("config.toml");
    fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

fn write_lib_rs(project_path: &Path, with_android: bool, basic: bool) {
    let mut content = if basic { LIB_RS_TEMPLATE.to_string() } else { LIB_RS_FRAMEWORK_TEMPLATE.to_string() };
    if !with_android {
        content = content.replace("\n#[cfg(target_os = \"android\")]\nuse android_activity::AndroidApp;\n", "");
        if let Some(start) = content.find("\n#[unsafe(no_mangle)]")
            && let Some(end_offset) = content[start..].find("\n}\n")
        {
            let end = start + end_offset + 3;
            content = format!("{}{}", &content[..start], &content[end..]);
        }
    }

    let path = project_path.join("src/lib.rs");
    fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

fn write_main_rs(project_path: &Path, crate_name: &str, basic: bool) {
    let content = if basic {
        MAIN_BIN_TEMPLATE.replace("vr_app", crate_name)
    } else {
        MAIN_BIN_FRAMEWORK_TEMPLATE.replace("vr_app", crate_name)
    };

    let path = project_path.join(format!("src/bin/main_{crate_name}.rs"));
    fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

fn write_gradle_files(project_path: &Path, project_name: &str, crate_name: &str) {
    let application_id = if project_name.contains('.') {
        project_name.replace('-', "_")
    } else {
        format!("com.stereokit.{crate_name}")
    };

    // Create gradle directories
    {
        let dir = &"app/src/main";
        fs::create_dir_all(project_path.join(dir)).unwrap_or_else(|e| panic!("Failed to create directory {dir}: {e}"));
    }

    // Copy res icons
    for (subdir, data) in [
        ("res/mipmap-ldpi", RES_ICON_LDPI),
        ("res/mipmap-mdpi", RES_ICON_MDPI),
        ("res/mipmap-hdpi", RES_ICON_HDPI),
        ("res/mipmap-xhdpi", RES_ICON_XHDPI),
        ("res/mipmap-xxhdpi", RES_ICON_XXHDPI),
        ("res/mipmap-xxxhdpi", RES_ICON_XXXHDPI),
    ] {
        fs::create_dir_all(project_path.join(subdir))
            .unwrap_or_else(|e| panic!("Failed to create directory {subdir}: {e}"));
        write_bytes(project_path, &format!("{subdir}/app_icon.png"), data);
    }

    // Root build.gradle
    write_file(project_path, "build.gradle", GRADLE_BUILD);

    // settings.gradle
    write_file(project_path, "settings.gradle", GRADLE_SETTINGS);

    // gradle.properties (substituted)
    let gradle_props = GRADLE_PROPERTIES
        .replace("${CARGO_LIBNAME}", crate_name)
        .replace("${APP_NAME}", project_name)
        .replace("${APPLICATION_ID}", &application_id);
    write_file(project_path, "gradle.properties", &gradle_props);

    // app/build.gradle
    write_file(project_path, "app/build.gradle", GRADLE_APP_BUILD);

    // app/src/main/AndroidManifest.xml
    write_file(project_path, "app/src/main/AndroidManifest.xml", GRADLE_MANIFEST);

    // MainActivity.java
    let package_dir = format!("app/src/main/java/{}", application_id.replace('.', "/"));
    fs::create_dir_all(project_path.join(&package_dir))
        .unwrap_or_else(|e| panic!("Failed to create directory {package_dir}: {e}"));
    let main_activity = GRADLE_MAIN_ACTIVITY
        .replace("${APPLICATION_ID}", &application_id)
        .replace("${CARGO_LIBNAME}", crate_name);
    write_file(project_path, &format!("{package_dir}/MainActivity.java"), &main_activity);

    // Gradle wrapper
    write_file(project_path, "gradlew", GRADLEW);
    set_executable(project_path, "gradlew");
    write_file(project_path, "gradlew.bat", GRADLEW_BAT);
    fs::create_dir_all(project_path.join("gradle/wrapper"))
        .unwrap_or_else(|e| panic!("Failed to create gradle/wrapper: {e}"));
    write_bytes(project_path, "gradle/wrapper/gradle-wrapper.jar", GRADLE_WRAPPER_JAR);
    write_file(project_path, "gradle/wrapper/gradle-wrapper.properties", GRADLE_WRAPPER_PROPS);
    write_file(project_path, "gradle/wrapper/action.yml", GRADLE_WRAPPER_ACTION);
    write_file(project_path, "gradle/libs.versions.toml", GRADLE_LIBS_VERSIONS);

    // logcat.cmd (used by getUid task)
    write_file(project_path, "logcat.cmd", GRADLE_LOGCAT_CMD);

    println!("Gradle files created.");
    println!();
    println!("  Next steps for Android builds:");
    println!("  1. Replace app icons in res/mipmap-*/app_icon.png with your own (see https://icon.kitchen)");
    println!("  2. Create a signing keystore and set credentials in ~/.gradle/gradle.properties");
}

fn write_file(project_path: &Path, relative: &str, content: &str) {
    let path = project_path.join(relative);
    fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

fn write_bytes(project_path: &Path, relative: &str, content: &[u8]) {
    let path = project_path.join(relative);
    fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

#[cfg(unix)]
fn set_executable(project_path: &Path, relative: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = project_path.join(relative);
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(&path, perms)
        .unwrap_or_else(|e| panic!("Failed to set permissions on {}: {e}", path.display()));
}

#[cfg(not(unix))]
fn set_executable(_project_path: &Path, _relative: &str) {}
