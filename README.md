## This project is a rust binding of the excellent [StereoKit](https://StereoKit.net) [project](https://github.com/StereoKit/StereoKit/)

![Screenshot](/StereoKit-rust.png)

Its purpose is to improve the previous rust project <https://github.com/MalekiRe/stereokit-rs>:
- by abandoning the automatic bindgen to avoid the multiplication of data types and eliminate costly transmutes.
- by implementing all the missing API calls and data types.
- by getting closer to the C# object model.
- by offering a framework based on winit/android-activity which takes up the ISTEPPERS of the C# framework.


This project is at an early stage so try it carefully. Right now, the only way to get the project is to get the source code from github.

### Download the source project:
* `git clone --recursive https://github.com/mvvvv/StereoKit-rust/`
* On linux get the following dev libraries : clang-18 lld-18 ninja-build libx11-dev libxfixes-dev libegl-dev libgbm-dev libfontconfig-dev.
* If you want to launch the demos then compile the shaders. From StereoKit-rust directory launch `cargo run --bin cargo-compile_sks`

### Run the project's demo on your PC's headset :
* Make sure you have [OpenXR installed](https://www.khronos.org/openxr/) with an active runtine.
* Launch[^1]: `cargo run  --example main_pc`

### Run the project's demo on your PC using the [simulator](https://stereokit.net/Pages/Guides/Using-The-Simulator.html) 
* Launch[^1]: `cargo run  --example main_pc -- --test`


### Run the project's demo on your Android headset (from a PC running Windows, Mac or Linux):
* Install [sdkmanager](https://developer.android.com/studio/command-line/sdkmanager)  (or Android Studio if you intend to use it). Set ANDROID_HOME environment variable to its path (this path contains the `build_tools` directory)
* Check that `adb` is connecting to your headset then set a valid NDK path into ANDROID_NDK_ROOT environment variable.
* Install: `cargo install cargo-apk` (cargo-xbuild has not been tested yet).
* Download: `rustup target add aarch64-linux-android` for most of the existing android headsets.
* Launch: `cargo apk run  --example main`

### Template to create your own project:
* git clone https://github.com/mvvvv/stereokit-template/
* If you don't clone the template project in the same directory than the StereoKit-rust project, you'll have to modify the path of the Stereokit-rust dependency.

### Use your own event manager (PC only)
The demo and template above, are using [winit](https://github.com/rust-windowing/winit) as an event manager and interface with the OS. If you want to use your own loop and event manager, have a look to [manual.rs](https://github.com/mvvvv/StereoKit-rust/blob/master/examples/manual.rs).
This is the shortest way to launch your first PCVR/PCMR program[^1]: `cargo run --example manual`

## Troubleshooting
Submit bugs on the [Issues tab](https://github.com/mvvvv/StereoKit-rust/issues), and ask questions in the [Discussions tab](https://github.com/mvvvv/StereoKit-rust/discussions)!

The project <https://github.com/StereoKit/StereoKit/> will give you many useful links (Discord/Twitter/Blog).


## Dependencies

This project was made possible thanks to the work of many talents on the following projects:
* [StereoKit](https://github.com/StereoKit/StereoKit/tree/cb6717aa8bc853e039bf3e0751cf4bff24c94910?tab=readme-ov-file#dependencies) which itself is based on valuable projects.
* [rust_mobile](https://github.com/rust-mobile) used for the android specific code.
* [winit](https://github.com/rust-windowing/winit) used for cross-platform management. 
* [openxrs](https://github.com/Ralith/openxrs) nice binding of OpenXR.
* bitflags.
* android_logger.
* this_error & anyerror.
* ... many others, more discreet, without which nothing would be possible.

[^1]: If you're using VsCode you'll see a corresponding launcher in launch.json to debug the app.