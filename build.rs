use cmake::Config;
use std::{env, fs, path::Path};

macro_rules! cargo_link {
    ($feature:expr) => {
        println!("cargo:rustc-link-lib={}", $feature);
    };
}

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

    let win_gnu_libs = env::var("SK_RUST_WIN_GNU_LIBS").unwrap_or_default();
    let win_gl = !env::var("SK_RUST_WINDOWS_GL").unwrap_or_default().is_empty();
    let skc_in_dll = cfg!(feature = "skc-in-dll");

    if win_gl {
        println!("cargo:info=Compiling with {} for {}/opengl with profile {}", target_env, target_os, profile);
    } else {
        println!("cargo:info=Compiling with {} for {} with profile {}", target_env, target_os, profile);
    }

    if target_os == "macos" {
        println!(
            "cargo:warning=You seem to be building for MacOS! We still enable builds so that rust-analyzer works, but this won't actually build StereoKit so it'll be pretty non-functional."
        );
        return;
    }

    // Build StereoKit, and tell rustc to link it.
    let mut cmake_config = Config::new("StereoKit");

    let profile_upper = if profile == "debug" {
        cmake_config.define("CMAKE_BUILD_TYPE", "Debug");
        "Debug"
    } else {
        "Release"
    };

    if !win_gnu_libs.is_empty() {
        cmake_config.define("CMAKE_SYSTEM_NAME", "Windows");
        if win_gl {
            cmake_config.cxxflag("-Wl,-allow-multiple-definition");
            cmake_config.define("__MINGW32__", "ON");
            cmake_config.define("WINDOWS_LIBS", "comdlg32;opengl32;");
        } else {
            cmake_config.define("__MINGW32__", "ON");
            cmake_config.define("WINDOWS_LIBS", "comdlg32;dxgi;d3d11;");
        }
    }

    if win_gl {
        cmake_config.define("SK_WINDOWS_GL", "ON");
    }

    if cfg!(feature = "force-local-deps") && env::var("FORCE_LOCAL_DEPS").is_ok() {
        println!("cargo:info=Force local deps !!");
        // Helper function to define optional dependencies
        fn define_if_exists(var_name: &str, cmake_var: &str, config: &mut Config) {
            if let Ok(value) = env::var(var_name) {
                config.define(cmake_var, value);
            }
        }

        define_if_exists("DEP_OPENXR_LOADER_SOURCE", "CPM_openxr_loader_SOURCE", &mut cmake_config);
        define_if_exists("DEP_MESHOPTIMIZER_SOURCE", "CPM_meshoptimizer_SOURCE", &mut cmake_config);
        define_if_exists("DEP_BASIS_UNIVERSAL_SOURCE", "CPM_basis_universal_SOURCE", &mut cmake_config);
        define_if_exists("DEP_SK_GPU_SOURCE", "CPM_sk_gpu_SOURCE", &mut cmake_config);
    }

    if target_family.as_str() == "windows" {
        if skc_in_dll {
            cmake_config.define("SK_BUILD_SHARED_LIBS", "ON");
        } else {
            cmake_config.define("SK_BUILD_SHARED_LIBS", "OFF");
        }
    } else {
        cmake_config.define("SK_BUILD_SHARED_LIBS", "OFF");
    }
    cmake_config.define("SK_BUILD_TESTS", "OFF").define("SK_PHYSICS", "OFF");
    if target_os == "android" {
        cmake_config.define("CMAKE_ANDROID_API", "32");
        cmake_config.define("CMAKE_INSTALL_INCLUDEDIR", "install");
        cmake_config.define("CMAKE_INSTALL_LIBDIR", "install");
        cmake_config.define("CMAKE_GENERATOR", "Ninja");
    }
    if cfg!(feature = "build-dynamic-openxr") {
        // When you need to build and use Khronos openxr loader use this feature:
        cmake_config.define("SK_DYNAMIC_OPENXR", "ON");
        cmake_config.define("SK_BUILD_OPENXR_LOADER", "ON");
    } else if cfg!(feature = "dynamic-openxr") {
        // When you need to ship your own openxr loader use this feature:
        cmake_config.define("SK_DYNAMIC_OPENXR", "ON");
    }

    let dst = cmake_config.build();

    let out_dir = env::var("OUT_DIR").unwrap(); //---must be equal to dst

    match target_family.as_str() {
        "windows" => {
            println!("cargo:rustc-link-search=native={}/lib", dst.display());
            println!("cargo:rustc-link-search=native={}/build/{}", dst.display(), profile_upper);

            cargo_link!("StereoKitC");

            if cfg!(debug_assertions) {
                // openxr-sys/linked wants libopenxr_loader so it asks for -Wl -lopenxr_loader in final ld
                cargo_link!("openxr_loaderd");
            } else {
                cargo_link!("openxr_loader");
            }

            cargo_link!("meshoptimizer");
            cargo_link!("windowsapp");
            cargo_link!("user32");
            cargo_link!("shell32");
            // test not really useful, just there to recall this annoying problem:
            if cfg!(windows) {
                cargo_link!("Comdlg32");
            } else {
                cargo_link!("comdlg32");
            }
            println!("cargo:rustc-link-search=native={}", dst.display());
            if target_env == "gnu" {
                if !skc_in_dll {
                    println!("cargo:rustc-link-search=native={}/build", dst.display());
                    println!("cargo:rustc-link-search=native={}/lib", dst.display());
                    println!("cargo:rustc-link-search=native={}", win_gnu_libs);
                    cargo_link!("gcc_eh");
                    cargo_link!("stdc++");
                    cargo_link!("meshoptimizer");
                } else {
                    //---- We have to extract the DLL i.e. ".\target\x86_64-pc-windows-gnu\debug\build\stereokit-rust-be362d37871b9048\out\build\StereoKitC.dll"
                    //---- and copy it to ".\target\x86_64-pc-windows-gnu\debug\deps\
                    //println!("cargo:rustc-link-search=native={}/build", dst.display());
                    println!("cargo:rustc-link-search=native={}", win_gnu_libs);
                    let deuleuleu = "libStereoKitC.dll";
                    let target_dir = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();
                    let deps_libs = target_dir.join("deps");
                    println!("cargo:rustc-link-search=native={}", deps_libs.to_str().unwrap());
                    println!("cargo:info=dst --> {:?}", dst);
                    let dest_file_dll = deps_libs.join(deuleuleu);
                    let file_dll = dst.join("build").join(deuleuleu);
                    println!("cargo:info={} is copied from here --> {:?}", deuleuleu, file_dll);
                    println!("cargo:info=                             to there --> {:?}", dest_file_dll);
                    let _lib_dll = fs::copy(file_dll, dest_file_dll).unwrap();
                }
            } else {
                //---- We have to extract the DLL i.e. ".\target\debug\build\stereokit-rust-be362d37871b9048\out\build\Debug\StereoKitC.dll"
                //---- and copy it to ".\target\debug\deps\
                let lib: String = "StereoKitC".into();
                let deuleuleu = lib.clone() + ".dll";
                let lib_lib = lib.clone() + ".lib";
                let lib_pdb = lib.clone() + ".pdb";
                let target_dir = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();
                let deps_libs = target_dir.join("deps");
                println!("cargo:info=dst --> {:?}", dst);
                //---Do we have a .dll ?
                let file_dll = dst.join("build").join(&profile).join(&deuleuleu);
                if file_dll.is_file() {
                    let dest_file_dll = deps_libs.join(&deuleuleu);
                    println!("cargo:info=StereoKitC.dll is copied from here --> {:?}", file_dll);
                    println!("cargo:info=                          to there --> {:?}", dest_file_dll);
                    let _lib_dll = fs::copy(file_dll, dest_file_dll).unwrap();
                }
                //---Do we have a .lib ?
                let file_lib = dst.join("build").join(&profile).join(&lib_lib);
                if file_lib.is_file() {
                    let dest_file_lib = deps_libs.join(&lib_lib);
                    println!("cargo:info=StereoKitC.lib is copied from here --> {:?}", file_lib);
                    println!("cargo:info=                          to there --> {:?}", dest_file_lib);
                    let _lib_dll = fs::copy(file_lib, dest_file_lib).unwrap();
                }
                //---Do we have a .pdb ?
                let file_pdb = dst.join("build").join(&profile).join(&lib_pdb);
                if file_pdb.is_file() {
                    let dest_file_pdb = deps_libs.join(&lib_pdb);
                    println!("cargo:info=StereoKitC.pdb is copied from here --> {:?}", file_pdb);
                    println!("cargo:info=                          to there --> {:?}", dest_file_pdb);
                    let _lib_dll = fs::copy(file_pdb, dest_file_pdb).unwrap();
                }
            }
        }
        "wasm" => {
            unimplemented!("sorry wasm isn't implemented yet");
        }
        "unix" => {
            println!("cargo:rustc-link-search=native={}/lib", dst.display());
            println!("cargo:rustc-link-search=native={}/lib64", dst.display());
            println!("cargo:rustc-link-search=native={}/build", dst.display());
            println!("cargo:rustc-link-search=native={}/install", dst.display());

            cargo_link!("StereoKitC");

            cargo_link!("stdc++");
            cargo_link!("openxr_loader");
            cargo_link!("meshoptimizer");
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
                println!("cargo:info=dst --> {:?}", dst);
                println!("cargo:info=Android runtime_libs are copied here --> {:?}", runtime_libs);
                assert!(target_dir.ends_with("target"));
                if let Err(_e) = fs::create_dir(&runtime_libs) {};
                runtime_libs = runtime_libs.join(&abi);
                if let Err(_e) = fs::create_dir(&runtime_libs) {};
                let dest_file_so = runtime_libs.join("libopenxr_loader.so");
                if cfg!(feature = "build-dynamic-openxr") {
                    let file_so = dst.join("lib/libopenxr_loader.so");
                    let _lib_o = fs::copy(file_so, dest_file_so).expect("Unable to copy libopenxr_loader.so");
                } else if let Err(_e) = fs::remove_file(dest_file_so) {
                }

                // // On Android, we must ensure that we're dynamically linking against the C++ standard library.
                // // For more details, see https://github.com/rust-windowing/android-ndk-rs/issues/167
                // // build tools do not add libc++_shared.so to the APK so we must do it ourselves
                // let host_tab = format!("{}-{}", env::consts::OS, env::consts::ARCH);
                // let target = env::var("TARGET").unwrap();
                // let ndk = env::var("ANDROID_NDK_ROOT").unwrap();

                // let libcxx = format!(
                //     "{}/toolchains/llvm/prebuilt/{}/sysroot/usr/lib/{}/libc++_shared.so",
                //     ndk, host_tab, target
                // );
                // println!("cargo:info=We copy {} to the {:?}", libcxx, runtime_libs);
                // let dest_file_so = runtime_libs.join("libc++_shared.so");
                // fs::copy(libcxx, dest_file_so).expect("Unable to copy libc++_shared.so");

                cargo_link!("dylib=c++");
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
}
