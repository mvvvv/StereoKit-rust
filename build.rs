use std::{env, fs, path::Path};
use cmake::Config;

macro_rules! cargo_link {
    ($feature:expr) => {
        println!("cargo:rustc-link-lib={}", $feature);
    };
}

fn main() {
    let target_os = var("CARGO_CFG_TARGET_OS").unwrap();
    let target_family = var("CARGO_CFG_TARGET_FAMILY").unwrap();

    if target_os == "macos" {
        println!("cargo:warning=You seem to be building for MacOS! We still enable builds so that rust-analyzer works, but this won't actually build StereoKit so it'll be pretty non-functional.");
        return;
    }

    // Build StereoKit, and tell rustc to link it.
    let mut cmake_config = Config::new("StereoKit");

    if cfg!(feature = "force-local-deps") && var("FORCE_LOCAL_DEPS").ok().is_some() {
        // Helper function to define optional dependencies
        fn define_if_exists(var_name: &str, cmake_var: &str, config: &mut Config) {
            if let Some(value) = var(var_name).ok() {
                config.define(cmake_var, value);
            }
        }

        define_if_exists("DEP_OPENXR_LOADER_SOURCE", "CPM_openxr_loader_SOURCE", &mut cmake_config);
        define_if_exists("DEP_MESHOPTIMIZER_SOURCE", "CPM_meshoptimizer_SOURCE", &mut cmake_config);
        define_if_exists("DEP_BASIS_UNIVERSAL_SOURCE", "CPM_basis_universal_SOURCE", &mut cmake_config);
        define_if_exists("DEP_SK_GPU_SOURCE", "CPM_sk_gpu_SOURCE", &mut cmake_config);
    }

    cmake_config
        .define("SK_BUILD_SHARED_LIBS", "OFF")
        .define("SK_BUILD_TESTS", "OFF")
        .define("SK_PHYSICS", "OFF");
    if target_os == "android" {
        cmake_config.define("CMAKE_ANDROID_API", "32");
        cmake_config.define("CMAKE_INSTALL_INCLUDEDIR", "install");
        cmake_config.define("CMAKE_INSTALL_LIBDIR", "install");
    }
    cmake_config.define("SK_DYNAMIC_OPENXR", if cfg!(feature = "dynamic-openxr") { "ON" } else { "OFF" });
    cmake_config.define("SK_BUILD_OPENXR_LOADER", if cfg!(feature = "build-dynamic-openxr") { "ON" } else { "OFF" });
    cmake_config.define("SK_BUILD_SHARED_LIBS", "OFF");

    let dst = cmake_config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-search=native={}/install", dst.display());
    cargo_link!("static=StereoKitC");
    match target_family.as_str() {
        "windows" => {
            if cfg!(debug_assertions) {
                cargo_link!("static=openxr_loaderd");
            } else {
                cargo_link!("static=openxr_loader");
            }
            cargo_link!("meshoptimizer");
            cargo_link!("windowsapp");
            cargo_link!("user32");
            cargo_link!("comdlg32");
            println!("cargo:rustc-link-search=native={}", dst.display());
        }
        "wasm" => {
            unimplemented!("sorry wasm isn't implemented yet");
        }
        "unix" => {
            if target_os == "macos" {
                panic!("Sorry, macos is not supported for stereokit.");
            }
            cargo_link!("stdc++");
            cargo_link!("openxr_loader");
            cargo_link!("static=meshoptimizer");
            if target_os == "android" {
                cargo_link!("android");
                cargo_link!("EGL");

                let mut abi = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
                if abi == "aarch64" {
                    abi = "arm64-v8a".to_string();
                }
                //---- A directory whose content is only used during the production of the APK (no need for DEBUG/RELEASE sub directory)
                //---- Copying from ./target/aarch64-linux-android/debug/build/stereokit-rust-1d044aba61d6313d/out/lib/libopenxr_loader.so
                //---- to   ./target/runtime_libs/
                let out_dir = env::var("OUT_DIR").unwrap(); //---must be equal to dst
                let target_dir = Path::new(&out_dir)
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap();
                let mut runtime_libs = target_dir.join("runtime_libs");
                println!("dst --> {:?}", dst);
                println!("Android runtime_libs are copied here --> {:?}", runtime_libs);
                assert!(target_dir.ends_with("target"));
                if let Err(_e) = fs::create_dir(&runtime_libs) {};
                runtime_libs = runtime_libs.join(&abi);
                if let Err(_e) = fs::create_dir(&runtime_libs) {};
                let dest_file_so = runtime_libs.join("libopenxr_loader.so");
                if cfg!(feature = "build-dynamic-openxr") {
                    let file_so = dst.join("lib/libopenxr_loader.so");
                    let _lib_o = fs::copy(file_so, dest_file_so).unwrap();
                } else if let Err(_e) = fs::remove_file(dest_file_so) {}
            } else {
                cargo_link!("X11");
                cargo_link!("Xfixes");
                cargo_link!("GL");
                cargo_link!("EGL");
                cargo_link!("gbm");
                cargo_link!("fontconfig");
            }
        }
        _ => {
            panic!("target family is unknown");
        }
    }

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=StereoKit/StereoKitC/stereokit.h");
    println!("cargo:rerun-if-changed=StereoKit/StereoKitC/stereokit_ui.h");

    // On Android, we must ensure that we're dynamically linking against the C++ standard library.
    // For more details, see https://github.com/rust-windowing/android-ndk-rs/issues/167
    use std::env::var;
    if var("TARGET").map(|target| target == "aarch64-linux-android").unwrap_or(false) {
        cargo_link!("dylib=c++");
    }
}
