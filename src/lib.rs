//! StereoKit-rust is a binding for the [StereoKit](https://StereoKit.net) C API.
//! If the name of this crate contains "_rust" (not great for a Rust crate, we agree) it is to emphasize the fact that
//! StereoKit is first and foremost a C, C++, C# project.
//! StereoKit allows you to create VR/MR applications with ease on every headset platforms that run OpenXR.
//!
//! <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/StereoKit-rust.png" alt="screenshot" width="300">
//!
//! [StereoKit-rust GitHub repository](https://github.com/mvvvv/StereoKit-rust/) /
//! [StereoKit GitHub repository](https://github.com/StereoKit/StereoKit/)
//!
//! # How to read this documentation
//! If you already know the name of what you are looking for, the fastest way to
//! find it is to use the <a href="#" onclick="window.searchState.focus();">search
//! bar</a> at the top of the page.
//! Otherwise, you may want to jump to one of these useful sections:
//! * [Installation](#installation)
//! * [Usage](#usage)
//! * [Examples](#examples)
//! * [How to build and test your application](#how-to-build-and-test-your-application)
//! * [StereoKit-rust modules](#modules)
//! * [StereoKit-rust Macros](#macros)
//!
//! # Installation:
//! StereoKit-rust is a binding and therefore requires some tools and libraries to compile StereoKitC:
//! ### On `Windows`:
//!   - Considering that you have already installed "`Visual Studio Build Tools 2022(Developpment Desktop C++)`" in order to
//!     have `Rust` compiling with `stable-????-pc-windows-msvc` toolchain.
//!   - Get the following tools and dev libraries : "`Git`", "`CMake`" and "`DotNet SDK v8+`".
//!
//! ### On `Linux`:
//!   - Considering that you have already installed `Rust` with `stable-?????-unknown-linux-gnu` toolchain and the linux package
//!     `build-essential`.
//!   - Get the following tools and dev libraries : `git` `clang` `cmake` `lld` `ninja-build` `libx11-dev`
//!     `libxfixes-dev` `libegl-dev` `libgbm-dev` `libfontconfig-dev` `libxkbcommon-x11-dev`.
//!
//! Installing the stereokit_rust tools with `cargo install -F no-event-loop stereokit-rust` should help you to check
//! the missing dependencies.
//!
//! # Usage
//! You have to chose between `event-loop` and `no-event-loop` features. The feature `no-event-loop` is the
//! lighter but you can't use the [`framework`].
//!
//! ## Features
//! - **`event-loop`**: Enables the framework with Winit integration for window management and event handling.
//! - **`no-event-loop`**: Lighter weight option without framework support.
//! - **`test-xr-mode`**: For testing - replaces `AppMode::Offscreen` with `AppMode::XR` in test macros to test with real XR devices.
//! - **`dynamic-openxr`**: Includes OpenXR loader dynamically for Android builds (APK).
//! - **`build-dynamic-openxr`**: Builds OpenXR loader from Khronos OpenXR project for Android builds (APK).
//!
//! Using `event-loop` your `Cargo.toml` should contain the following lines:
//! ```toml
//! [lib]
//! crate-type = ["lib", "cdylib"]
//!
//! [dependencies]
//! stereokit-rust = { version = "0.4.0", features= ["event-loop"] }
//! winit = { version = "0.30", features = [ "android-native-activity" ] }
//!
//! [target.'cfg(target_os = "android")'.dependencies]
//! stereokit-rust = { version = "0.4.0" , features = ["event-loop", "build-dynamic-openxr"] }
//! log = "0.4"
//! android_logger = "0.15"
//! ndk-context = "0.1.1"
//! ndk = "0.9.0"
//! ```
//!
//! # Examples
//! Here is a simple "Hello World" StereoKit-rust app for all platforms:
//! ```bash
//! cargo new --lib vr_app
//! cd vr_app
//! ```
//!
//! In `src/bin/main_vr_app.rs`, if you intend to build a PC VR/MR app:
//! ```ignore
//! #[allow(dead_code)]
//! #[cfg(not(target_os = "android"))]
//! fn main() {
//!     use stereokit_rust::sk::{SkSettings, Sk};
//!     use vr_app::the_main;
//!     // Initialize StereoKit with default settings
//!     let mut settings = SkSettings::default();
//!     settings.app_name("Test");
//!     # settings.mode(stereokit_rust::sk::AppMode::Offscreen);
//!     let (sk, event_loop) = settings.init_with_event_loop()
//!         .expect("Should initialize StereoKit");
//!     the_main(sk, event_loop);
//!     Sk::shutdown();
//! }
//!
//! #[allow(dead_code)]
//! #[cfg(target_os = "android")]
//! //fake main fn for android as entry is lib.rs/android_main(...)
//! fn main() {}
//!
//! # use stereokit_rust::prelude::*;
//! # use winit::event_loop::EventLoop;
//! # pub fn the_main(sk: Sk, event_loop: EventLoop<StepperAction>) {}
//! ```
//!
//! In `src/lib.rs` where you can remove the `target_os = "android" code` if you don't want to build for Android:
//! ```ignore
//! use stereokit_rust::{framework::SkClosures, prelude::*, sk::Sk, ui::Ui};
//! use winit::event_loop::EventLoop;
//!
//! #[cfg(target_os = "android")]
//! use winit::platform::android::activity::AndroidApp;
//!
//! #[unsafe(no_mangle)]
//! #[cfg(target_os = "android")]
//! pub fn android_main(app: AndroidApp) {
//!     use stereokit_rust::sk::SkSettings;
//!     // Initialize StereoKit with default settings
//!     let mut settings = SkSettings::default();
//!     settings.app_name("Test");
//!     android_logger::init_once(
//!         android_logger::Config::default()
//!               .with_max_level(log::LevelFilter::Debug)
//!               .with_tag("STKit-rs"),
//!     );
//!     let (sk, event_loop) = settings.init_with_event_loop(app).unwrap();
//!     the_main(sk, event_loop);
//! }
//!
//! /// Main function for All!
//! pub fn the_main(sk: Sk, event_loop: EventLoop<StepperAction>) {
//!     // Create a grabbable window with a button to exit the application
//!     let mut window_pose = Ui::popup_pose([0.0, -0.4, 0.0]);
//!     // Main loop
//!     SkClosures::new(sk, |sk, _token| {
//!         // Exit button
//!         Ui::window_begin("Hello world!", &mut window_pose, None, None, None);
//!         if Ui::button("Exit", None) {
//!             sk.quit(None)
//!         }
//!         Ui::window_end();
//!     })
//!     .run(event_loop);
//! }
//! ```
//!
//! Hundreds of examples (which are also unit tests) are available in this documentation. If you like to learn by
//! examples, check out  the modules in the following order: [`sk`], [`mesh`], [`model`], [`maths`], [`ui`], [`framework`],
//! [`tools`], [`sound`], [`interactor`], [`system`], [`permission`] [`material`], [`shader`], [`tex`], [`sprite`], [`font`], [`render_list`].
//!
//! # How to build and test your application:
//!
//! * [Building your PC VR/MR app](#building-your-pc-vrmr-app).
//! * [Building your Android VR/MR app](#building-your-android-vrmr-app).
//! * [Building your Windows GNU PC VR/MR app](#building-your-windows-gnu-pc-vrmr-app).
//! * [Building your Linux AARCH64 PC VR/MR app](#building-your-linux-aarch64-pc-vrmr-app).
//! * [Building your Linux X86_64 PC VR/MR app](#building-your-linux-x86_64-pc-vrmr-app).
//!
//! ## Building your PC VR/MR app:
//! * Launch `cargo run --bin main_vr_app` to compile and run your app in debug mode on your PC with or without a headset.
//!   (using Wayland on Linux may require to unset temporarily the DISPLAY variable: `DISPLAY= cargo run`)
//! * Launch `cargo build_sk_rs --bin main_vr_app <build_directory>` to compile your app and assets in release mode for your PC.
//!
//! To test with your headset, make sure you have [OpenXR installed](https://www.khronos.org/openxr/) with an active
//! runtine (SteamVR, Monado, WiVRn, ALVR ...).
//!
//! ## Building your Android VR/MR app:
//! This can be done from a PC running Windows, Mac or Linux:
//! * Install [sdkmanager](https://developer.android.com/studio/command-line/sdkmanager) (or Android Studio if you
//!   intend to use it). You'll need a Java JDK (v17 is fine).
//! * Using sdkmanager, install platform-tools(v32), latest build-tools and the latest ndk.
//! * Set ANDROID_HOME environment variable to its path (this path contains the `build_tools` directory).
//! * Set the NDK path (which ends with it's version number) into the ANDROID_NDK_ROOT environment variable.
//! * Install [Ninja](https://ninja-build.org/)
//! * Check that `adb` ($ANDROID_HOME/platform_tools/adb) is connecting to your headset.
//! * Download the target: `rustup target add aarch64-linux-android` for most existing android headsets.
//! * Create a keystore for signing your app (using keytool or Android Studio).
//! ##### If you don't need some java/kotlin code, you can use cargo-apk  (cargo-xbuild is an alternative but lacks some documentation):
//!   - Install: `cargo install cargo-apk`.
//!   - The manifest file will be generated from the `Cargo.toml` (see the `package.metadata.android` section). Here are
//!     some examples:
//!     - [StereoKit-template](https://github.com/mvvvv/stereokit-template/blob/main/Cargo.toml#L27)
//!     - [StereoKit-rust](https://github.com/mvvvv/StereoKit-rust/blob/master/Cargo.toml#L77)
//!   - Create a res directory with the icons of your app (i.e. with <https://icon.kitchen>)
//!   - Set the path and password to your keystore in the `Cargo.toml` [package.metadata.android.signing.release] or
//!     in the `CARGO_APK_RELEASE_KEYSTORE` and  `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` environment variables.
//!   - Launch the debug on your headset: `cargo apk run --lib`
//!   - Generate the release apk: `cargo apk build --lib --release`. The apk will be in `target/release/apk/`.
//! ##### Otherwise, you have to use Gradle with cargo-ndk:
//!   - Install: `cargo install cargo-ndk`.
//!   - Clone or extract a ZIP of [gradle template](https://github.com/mvvvv/stereokit-template/tree/gradle).
//!   - Name your project in the `package.name` entry in `Cargo.toml`.
//!   - Set `cargo.libName` (same as `package.name` from `Cargo.toml`), `android.applicationId` and `android.main` in
//!     `gradle.properties`.
//!   - In `app/src/main/AndroidManifest.xml` delete or modify the path and package name of `MainActivity.java` (your
//!     choice impacts android.main ↑ and android:hasCode attribute).
//!   - Replace the content of the res directory with the icons of your app (i.e. with <https://icon.kitchen>)
//!   - Store your keystore values in one of the hidden gradle properties files (ie. `~/.gradle/gradle.properties`)
//!     to store and forget the confidential values:
//!     - RELEASE_STORE_FILE=/home/**/**/my_release_key.keystore
//!     - RELEASE_STORE_PASSWORD=******
//!     - RELEASE_KEY_ALIAS=*****
//!     - RELEASE_KEY_PASSWORD=******
//!   - If any, remove the .git folder.
//!   - Launch the debug on your connected headset:
//!     - On Windows, launch: `./gradlew.bat run && cmd /c logcat.cmd` or `(./gradlew.bat run) -and (cmd /c logcat.cmd)`
//!     - On others, launch: `./gradlew run && ./logcat.cmd`
//!   - Generate the release apk: `./gradlew buildRelease`. The apk will be in `app/build/outputs/apk/release`
//!
//! ## Building your Windows GNU PC VR/MR app:
//! Thanks to Steam Proton, you can run your Windows exe on Linux. It's even better than native build thanks to D3D11
//! to Vulkan translation. Knowing that, we work to build Windows .exe files on Linux using GNU toolchain.
//!
//! Build your app for Windows_x64 using GNU toolchain from Linux and Windows (and probably Mac):
//! * Install mingw-w64 (MSYS2 on windows).
//! * Add the `Rust` target gnu for windows:`rustup target add x86_64-pc-windows-gnu`
//! * On 'Non Windows OS': we need wine to compile the shaders:
//!   - Add i386 architecture (i.e. `sudo dpkg --add-architecture i386` on Ubuntu).
//!   - Install wine and winetricks.
//!   - Install needed tools and libs: `winetricks corefonts d3dx9 d3dcompiler_47 dxvk`.
//! * Create a directory where necessary libs will be stored (i.e. ../x64-mingw-libs/) then add a link to the DLLs or
//!   static libs (*.a) the build will need after or during its creation. Example on Ubuntu 24.XX:
//!   - If you want to use DLLs:
//!      - `ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libgcc_s_seh-1.dll ../x64-mingw-libs/`
//!      - `ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libstdc++-6.dll ../x64-mingw-libs/`
//!   - If you want to use static libs:
//!      - `ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libgcc_eh.a ../x64-mingw-libs/`
//!      - `ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libstdc++.a ../x64-mingw-libs/`
//! * Launch: `cargo build_sk_rs --bin main_vr_app --x64-win-gnu ../x64-mingw-libs/ <the path of your exportable repository>`
//! * To run your_app.exe on Linux:
//!   - Add a non-steam game to your library then launch it when WiVRn or SteamVR are started.
//!   - If you only need the simulator: `wine your_app.exe`.
//!
//! ## Building your Linux aarch64 PC VR/MR app:
//! If you are on aarch64 Linux, you just have to follow the instructions in [`Building your PC VR/MR app`](#building-your-pc-vrmr-app).
//! If you are on a x86_64 architecture you are able to cross-compile your app for aarch64:
//! * Install g++-aarch64-linux-gnu
//! * Get the libraries `libx11-dev:arm64` `libxfixes-dev:arm64` `libegl-dev:arm64` `libgbm-dev:arm64` `libfontconfig-dev:arm64`.
//!   On Ubuntu 24:XX this can be done by adding a foreign architecture `dpkg --add-architecture arm64` with depot
//!   `http://ports.ubuntu.com/ubuntu-ports`. To avoid errors during `apt update` you'll have to specify the architectures
//!   of all depots in `/etc/apt/sources.list.d/ubuntu.sources`
//! * Add the rust target aarch64 for Linux:`rustup target add aarch64-unknown-linux-gnu`
//! * Add a section `[target.aarch64-unknown-linux-gnu]` in your config.toml for setting `linker = "aarch64-linux-gnu-gcc"`
//! * Launch `cargo build_sk_rs --bin main_vr_app --aarch64-linux <the path of your exportable repository>`
//!
//! ## Building your Linux x86_64 PC VR/MR app:
//! If you are on x86_64 Linux, you just have to follow the instructions in [`Building your PC VR/MR app`](#building-your-pc-vrmr-app).
//! If you are on aarch64 architecture you should be able to cross-compile for x86_64:
//! (This hasn't been tested yet, if you are interested in testing it, please let us now)
//! * Install g++-x86-64-linux-gnu
//! * Get the libraries `libx11-dev:amd64` `libxfixes-dev:amd64` `libegl-dev:amd64` `libgbm-dev:amd64` `libfontconfig-dev:amd64`.
//!   On Ubuntu 24:XX this can be done by adding a foreign architecture `dpkg --add-architecture amd64` with depot
//!   `http://ports.ubuntu.com/ubuntu-ports`. To avoid errors during `apt update` you'll have to specify the architectures
//!   of all depots in `/etc/apt/sources.list.d/ubuntu.sources`
//! * Add the rust target aarch64 for linux:`rustup target add x86_64-unknown-linux-gnu`
//! * Add a section `[target.x86_64-unknown-linux-gnu]` in your config.toml for setting `linker = "x86_64-linux-gnu-gcc"`
//! * Launch `cargo build_sk_rs --bin main_vr_app --x64-linux <the path of your exportable repository>`.

use std::{ffi::NulError, path::PathBuf};

#[cfg(feature = "event-loop")]
pub use stereokit_macros::IStepper;

pub use stereokit_macros::include_asset_tree;

#[cfg(feature = "event-loop")]
pub use stereokit_macros::test_init_sk_event_loop as test_init_sk;
#[cfg(feature = "no-event-loop")]
pub use stereokit_macros::test_init_sk_no_event_loop as test_init_sk;

pub use stereokit_macros::offscreen_mode_stop_here;
pub use stereokit_macros::xr_mode_stop_here;

#[cfg(feature = "event-loop")]
pub use stereokit_macros::test_screenshot_event_loop as test_screenshot;
#[cfg(feature = "no-event-loop")]
pub use stereokit_macros::test_screenshot_no_event_loop as test_screenshot;

#[cfg(feature = "event-loop")]
pub use stereokit_macros::test_steps_event_loop as test_steps;
#[cfg(feature = "no-event-loop")]
pub use stereokit_macros::test_steps_no_event_loop as test_steps;

/// Some of the errors you might encounter when using StereoKit-rust.
use thiserror::Error;

/// Anchor related structs and functions.
///
/// With examples which are also unit tests.
pub mod anchor;

/// Font related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Font](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/font.jpeg)](font::Font)
pub mod font;

/// A namespace containing features exclusive to the rust bindings for StereoKit.
///
/// These are higher level pieces of functionality that do not necessarily adhere to the same goals and restrictions as
/// StereoKit’s core functionality does. This corresponds to the C# namespace:
/// <https://stereokit.net/Pages/StereoKit.Framework.html>
/// - An event loop manager based on Winit.
/// - HandMenuRadial related structs, enums and functions.
///
/// At the core of this framework is the [`crate::IStepper`] derive macro, which allows you to create a stepper that can
/// be run in the event loop.
/// ## Examples
/// which are also unit tests:
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ font::Font, maths::{Matrix, Quat, Vec3},
///                       system::{Text, TextStyle}, util::named_colors};
///
/// #[derive(IStepper)]
/// pub struct MyStepper {
///     id: StepperId,
///     sk_info: Option<Rc<RefCell<SkInfo>>>,
///
///     transform: Matrix,
///     pub text: String,
///     text_style: Option<TextStyle>,
/// }
/// unsafe impl Send for MyStepper {}
/// impl Default for MyStepper {
///     fn default() -> Self {
///         Self {
///             id: "MyStepper".to_string(),
///             sk_info: None,
///
///             transform: Matrix::IDENTITY,
///             text: "IStepper\nderive\nmacro".to_owned(),
///             text_style: None,
///         }
///     }
/// }
/// impl MyStepper {
///     fn start(&mut self) -> bool {
///         self.transform = Matrix::t_r([0.05, 0.0, -0.2], [0.0, 200.0, 0.0]);
///         self.text_style = Some(Text::make_style(Font::default(), 0.3, named_colors::RED));
///         true
///     }
///     fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}
///     fn draw(&mut self, token: &MainThreadToken) {
///         Text::add_at(token, &self.text, self.transform, self.text_style,
///                      None, None, None, None, None, None);
///     }
/// }
///
/// sk.send_event(StepperAction::add_default::<MyStepper>("My_Basic_Stepper_ID"));
///
/// filename_scr = "screenshots/istepper_macro.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     // No code here as we only use MyStepper
/// );
/// ```
/// [![IStepper derive macro](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/istepper_macro.jpeg)](IStepper)
///
/// [![SkClosures](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sk_closures.jpeg)](framework::SkClosures)
/// [![IStepper](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/a_stepper.jpeg)](framework::IStepper)
/// [![StepperAction](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/stepper_actions.jpeg)](framework::StepperAction)
/// [![Steppers](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/steppers.jpeg)](framework::Steppers)
/// [![StepperClosures](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/stepper_closures.jpeg)](framework::StepperClosures)
pub mod framework;

/// Material specific structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Material](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/materials.jpeg)](material::Material)
/// [![Material Transparency](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/material_transparency.jpeg)](material::Material::transparency)
/// [![Material Face Cull](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/material_face_cull.jpeg)](material::Material::face_cull)
/// [![Material Parameter Info](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/param_infos.jpeg)](material::ParamInfos)
/// [![Material Parameter Info with id](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/param_infos_with_id.jpeg)](material::ParamInfos::set_data_with_id)
pub mod material;

/// Vec2, 3 and4, Quat and Matrix, Bounds, Plane and Ray related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Matrix](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/matrix.jpeg)](maths::Matrix)
/// [![Bounds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/bounds.jpeg)](maths::Bounds)
/// [![Plane](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/plane.jpeg)](maths::Plane)
/// [![Pose](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/pose.jpeg)](maths::Pose)
/// [![Sphere](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sphere.jpeg)](maths::Sphere)
/// [![Ray](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ray.jpeg)](maths::Ray)
/// [![Intersect Meshes](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/intersect_meshes.jpeg)](maths::Ray::intersect_mesh)
/// [![Intersect Model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/intersect_model.jpeg)](maths::Ray::intersect_model)
pub mod maths;

/// Mesh related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/meshes.jpeg)](mesh::Mesh)
/// [![Vertex](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/basic_mesh.jpeg)](mesh::Vertex)
/// [![Mesh bounds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_bounds.jpeg)](mesh::Mesh::bounds)
/// [![Mesh set_verts](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_verts.jpeg)](mesh::Mesh::set_verts)
/// [![Mesh set_inds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_inds.jpeg)](mesh::Mesh::set_inds)
/// [![Mesh draw](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_draw.jpeg)](mesh::Mesh::draw)
/// [![Mesh intersect](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_intersect.jpeg)](mesh::Mesh::intersect)
pub mod mesh;

/// Model specific structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model.jpeg)](model::Model)
/// [![Model from memory](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_memory.jpeg)](model::Model::from_memory)
/// [![Model from file](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_file.jpeg)](model::Model::from_file)
/// [![Model from mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_mesh.jpeg)](model::Model::from_mesh)
/// [![Model bounds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_bounds.jpeg)](model::Model::bounds)
/// [![Model draw](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_draw.jpeg)](model::Model::draw)
/// [![Model draw with material](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_draw_with_material.jpeg)](model::Model::draw_with_material)
/// [![Model intersect](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_intersect.jpeg)](model::Model::intersect)
/// [![Model Anims](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/anims.jpeg)](model::Anims)
/// [![Model Nodes](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_nodes.jpeg)](model::Nodes)
/// [![ModelNode](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_node.jpeg)](model::ModelNode)
pub mod model;

/// Permission related structs, enums and functions for managing cross-platform permissions.
pub mod permission;

/// Prelude for StereoKit-rust. The basis for all StereoKit-rust programs.
pub mod prelude;

/// RenderList related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![RenderList](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list.jpeg)](render_list::RenderList)
/// [![RenderList add mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_mesh.jpeg)](render_list::RenderList::add_mesh)
/// [![RenderList add model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_model.jpeg)](render_list::RenderList::add_model)
/// [![RenderList draw now](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_draw_now.jpeg)](render_list::RenderList::draw_now)
/// [![RenderList push](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_push.jpeg)](render_list::RenderList::push)
pub mod render_list;

/// Shader specific structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Shader](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/shaders.jpeg)](shader::Shader)
pub mod shader;

/// Sk related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Sk basic example](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sk_basic_example.jpeg)](sk::SkSettings::init_with_event_loop)
#[cfg(feature = "event-loop")]
pub mod sk;

/// StereoKit-rust specific structs, enums and functions.
#[cfg(feature = "no-event-loop")]
pub mod sk;

/// Sound specific structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Sound](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound.jpeg)](sound::Sound)
/// [![SoundInst](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound_inst.jpeg)](sound::SoundInst)
pub mod sound;

/// Sprite related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Sprite](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite.jpeg)](sprite::Sprite)
/// [![Sprite from Tex](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_from_tex.jpeg)](sprite::Sprite::from_tex)
/// [![Sprite from File](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_from_file.jpeg)](sprite::Sprite::from_file)
/// [![Sprite draw](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_draw.jpeg)](sprite::Sprite::draw)
///
/// [![Sprite grid](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_grid.jpeg)](sprite::Sprite::grid)
/// [![Sprite list](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_list.jpeg)](sprite::Sprite::list)
/// [![Sprite arrow left](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_left.jpeg)](sprite::Sprite::arrow_left)
/// [![Sprite arrow right](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_right.jpeg)](sprite::Sprite::arrow_right)
/// [![Sprite arrow up](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_up.jpeg)](sprite::Sprite::arrow_up)
/// [![Sprite arrow down](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_down.jpeg)](sprite::Sprite::arrow_down)
/// [![Sprite radio off](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_radio_off.jpeg)](sprite::Sprite::radio_off)
/// [![Sprite radio on](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_radio_on.jpeg)](sprite::Sprite::radio_on)
/// [![Sprite toggle off](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_toggle_off.jpeg)](sprite::Sprite::toggle_off)
/// [![Sprite toggle on](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_toggle_on.jpeg)](sprite::Sprite::toggle_on)
/// [![Sprite backspace](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_backspace.jpeg)](sprite::Sprite::backspace)
/// [![Sprite close](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_close.jpeg)](sprite::Sprite::close)
/// [![Sprite shift](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_shift.jpeg)](sprite::Sprite::shift)
pub mod sprite;

/// Interactor related structs, enums and functions.
///
pub mod interactor;

/// Sprite specific structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Assets](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/assets.jpeg)](system::Assets)
/// [![Assets block for priority](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/assets_block_for_priority.jpeg)](system::Assets::block_for_priority)
/// [![Hierarchy](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/hierarchy.jpeg)](system::Hierarchy)
/// [![Hierarchy ray](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/hierarchy_ray.jpeg)](system::Hierarchy::to_local_ray)
/// [![Hand](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/hand.jpeg)](system::Hand)
/// [![Controller](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/controller.jpeg)](system::Controller)
/// [![Lines](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/lines.jpeg)](system::Lines)
/// [![Microphone](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/microphone.jpeg)](system::Microphone)
/// [![Renderer](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/renderer.jpeg)](system::Renderer)
/// [![Screenshots capture](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/screenshot_capture.jpeg)](system::Renderer::screenshot_capture)
/// [![TextStyle](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/text_style.jpeg)](system::TextStyle)
/// [![Text](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/text.jpeg)](system::Text)
pub mod system;

/// Tex related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Tex](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex.jpeg)](tex::Tex)
/// [![Tex from file](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex_from_file.jpeg)](tex::Tex::from_file)
/// [![Tex gen_particle](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex_gen_particle.jpeg)](tex::Tex::gen_particle)
/// [![SHCubemap](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sh_cubemap.jpeg)](tex::SHCubemap)
pub mod tex;

/// Many `non-canonical`` tools related structs, enums and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![FileBrowser](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/file_browser.jpeg)](tools::file_browser::FileBrowser)
/// [![FlyOver](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/fly_over.jpeg)](tools::fly_over::FlyOver)
/// [![Log window](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/log_window.jpeg)](tools::log_window::LogWindow)
/// [![Hud Notification](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/hud_notification.jpeg)](tools::notif::HudNotification)
/// [![Screenshot viewer](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/screenshot_viewer.jpeg)](tools::screenshot::ScreenshotViewer)
/// [![Title](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/title.jpeg)](tools::title::Title)
pub mod tools;

/// The UI module is a collection of functions and structs that allow you to create a user interface.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Ui](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui.jpeg)](ui::Ui)
/// [![Ui color_scheme](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_color_scheme.jpeg)](ui::Ui::color_scheme)
/// [![Ui button](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_button.jpeg)](ui::Ui::button)
/// [![Ui button_img](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_button_img.jpeg)](ui::Ui::button_img)
/// [![Ui button_round](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_button_round.jpeg)](ui::Ui::button_round)
/// [![Ui handle](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_handle.jpeg)](ui::Ui::handle)
/// [![Ui handle_begin](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_handle_begin.jpeg)](ui::Ui::handle_begin)
/// [![Ui hseparator](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_hseparator.jpeg)](ui::Ui::hseparator)
/// [![Ui hslider](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_hslider.jpeg)](ui::Ui::hslider)
/// [![Ui vslider](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_vslider.jpeg)](ui::Ui::vslider)
/// [![Ui slider_behavior](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_slider_behavior.jpeg)](ui::Ui::slider_behavior)
/// [![Ui image](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_image.jpeg)](ui::Ui::image)
/// [![Ui input](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_input.jpeg)](ui::Ui::input)
/// [![Ui label](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_label.jpeg)](ui::Ui::label)
/// [![Ui layout_area](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_layout_area.jpeg)](ui::Ui::layout_area)
/// [![Ui layout_push](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_layout_push.jpeg)](ui::Ui::layout_push)
/// [![Ui model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_model.jpeg)](ui::Ui::model)
/// [![Ui panel_at](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_panel_at.jpeg)](ui::Ui::panel_at)
/// [![Ui panel_begin](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_panel_begin.jpeg)](ui::Ui::panel_begin)
/// [![Ui progress_bar_at](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_progress_bar_at.jpeg)](ui::Ui::progress_bar_at)
/// [![Ui push_surface](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_push_surface.jpeg)](ui::Ui::push_surface)
/// [![Ui push_text_style](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_push_text_style.jpeg)](ui::Ui::push_text_style)
/// [![Ui push_tint](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_push_tint.jpeg)](ui::Ui::push_tint)
/// [![Ui gen_quadrant_mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_gen_quadrant_mesh.jpeg)](ui::Ui::gen_quadrant_mesh)
/// [![Ui radio button](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_radio.jpeg)](ui::Ui::radio_img)
/// [![Ui toggle](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_toggle.jpeg)](ui::Ui::toggle)
/// [![Ui draw_element](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_draw_element.jpeg)](ui::Ui::draw_element)
/// [![Ui set_theme_color](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_set_theme_color.jpeg)](ui::Ui::set_theme_color)
/// [![Ui text](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_text.jpeg)](ui::Ui::text)
/// [![Ui window](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_window.jpeg)](ui::Ui::window_begin)
pub mod ui;

/// Many utility structs, enums and functions.
pub mod util;

/// Some of the errors you might encounter when using StereoKit-rust.
#[derive(Error, Debug)]
pub enum StereoKitError {
    #[error("unable to create model from file path {0}")]
    ModelFile(String),
    #[error("unable to find model with id {0}")]
    ModelFind(String),
    #[error("failed to create model {0} from memory for reason {1}")]
    ModelFromMem(String, String),
    #[error("failed to create model {0} from file for reason {1}")]
    ModelFromFile(PathBuf, String),
    #[error("failed to generate mesh {0}")]
    MeshGen(String),
    #[error("failed to find mesh {0}")]
    MeshFind(String),
    #[error("failed to convert to CString {0} in mesh_find")]
    MeshCString(String),
    #[error("failed to convert to CString {0} in tex_find")]
    TexCString(String),
    #[error("failed to find tex {0}")]
    TexFind(String),
    #[error("failed to copy tex {0}")]
    TexCopy(String),
    #[error("failed to create a tex from raw memory")]
    TexMemory,
    #[error("failed to create a tex from file {0} for reason {1}")]
    TexFile(PathBuf, String),
    #[error("failed to create a tex from multiple files {0} for reason {1}")]
    TexFiles(PathBuf, String),
    #[error("failed to create a tex from color {0} for reason {1}")]
    TexColor(String, String),
    #[error("failed to create a tex rendertarget {0} for reason {1}")]
    TexRenderTarget(String, String),
    #[error("failed to find font {0} for reason {1}")]
    FontFind(String, String),
    #[error("failed to create font from file {0} for reason {1}")]
    FontFile(PathBuf, String),
    #[error("failed to create font from multiple files {0} for reason {1}")]
    FontFiles(String, String),
    #[error("failed to create font family {0} for reason {1}")]
    FontFamily(String, String),
    #[error("failed to find shader {0} for reason {1}")]
    ShaderFind(String, String),
    #[error("failed to create shader from file {0} for reason {1}")]
    ShaderFile(PathBuf, String),
    #[error("failed to create shader from raw memory")]
    ShaderMem,
    #[error("failed to find material {0} for reason {1}")]
    MaterialFind(String, String),
    #[error("failed to create sprite from texture")]
    SpriteCreate,
    #[error("failed to create sprite from file {0}")]
    SpriteFile(PathBuf),
    #[error("failed to find sprite {0} for reason {1}")]
    SpriteFind(String, String),
    #[error("failed to find sound {0} for reason {1}")]
    SoundFind(String, String),
    #[error("failed to find render list {0} for reason {1}")]
    RenderListFind(String, String),
    #[error("failed to create sound from file {0}")]
    SoundFile(PathBuf),
    #[error("failed to create sound streaming {0}")]
    SoundCreate(String),
    #[error("failed to create anchor {0}")]
    AnchorCreate(String),
    #[error("failed to find anchor {0} for reason {1}")]
    AnchorFind(String, String),
    #[error("failed to init stereokit with settings {0}")]
    SkInit(String),
    #[cfg(feature = "event-loop")]
    #[error("failed to init stereokit event_loop")]
    SkInitEventLoop(#[from] winit::error::EventLoopError),
    #[error("failed to get a string from native C {0}")]
    CStrError(String),
    #[error("failed to read a file {0}: {1}")]
    ReadFileError(PathBuf, String),
    #[error("failed to write a file {0}: {1}")]
    WriteFileError(PathBuf, String),
    #[error("Directory {0} do not exist or is not a directory")]
    DirectoryError(String),
    #[error(transparent)]
    Other(#[from] NulError),
}
