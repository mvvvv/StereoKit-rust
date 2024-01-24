use std::env;

macro_rules! cargo_cmake_feat {
    ($feature:literal) => {
        if cfg!(feature = $feature) {
            "ON"
        } else {
            "OFF"
        }
    };
}
macro_rules! cargo_link {
    ($feature:expr) => {
        println!("cargo:rustc-link-lib={}", $feature);
    };
}
fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();

    // Build StereoKit, and tell rustc to link it.
    let mut cmake_config = cmake::Config::new("StereoKit");
    cmake_config.define("SK_BUILD_SHARED_LIBS", "OFF");
    cmake_config.define("SK_BUILD_TESTS", "OFF");
    cmake_config.define("SK_LINUX_EGL", cargo_cmake_feat!("linux-egl"));
    cmake_config.define("SK_PHYSICS", cargo_cmake_feat!("physics")); // cannot get this to work on windows.
    if target_os == "android" {
        cmake_config.define("CMAKE_ANDROID_API", "25");
        //cmake_config.define("ANDROID_LIBRARY","there");
        //cargo clcmake_config.define("ANDROID_LOG_LIBRARY","there");
        //cmake_config.define("ANDROID", "TRUE");
    }

    let dst = cmake_config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    cargo_link!("static=StereoKitC");
    match target_family.as_str() {
        "windows" => {
            if cfg!(debug_assertions) {
                cargo_link!("static=openxr_loaderd");
            } else {
                cargo_link!("static=openxr_loader");
            }
            cargo_link!("windowsapp");
            cargo_link!("user32");
            cargo_link!("comdlg32");
            println!("cargo:rustc-link-search=native={}", dst.display());
            if cfg!(feature = "physics") {
                println!("cargo:rustc-link-lib=static=build/_deps/reactphysics3d-build/Debug/reactphysics3d");
            }
            //cargo_link!("static=reactphysics3d");
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
            if target_os == "android" {
                cargo_link!("android");
                cargo_link!("EGL");
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
    // println!("cargo:rerun-if-changed=src/static-wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=StereoKit/StereoKitC/stereokit.h");
    println!("cargo:rerun-if-changed=StereoKit/StereoKitC/stereokit_ui.h");

    // On Android, we must ensure that we're dynamically linking against the C++ standard library.
    // For more details, see https://github.com/rust-windowing/android-ndk-rs/issues/167
    use std::env::var;
    if var("TARGET").map(|target| target == "aarch64-linux-android").unwrap_or(false) {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
