## This project is a rust binding of the excellent [StereoKit](https://StereoKit.net) [project](https://github.com/StereoKit/StereoKit/)

![Screenshot](/StereoKit-rust.png)

Its purpose is to improve the previous rust project <https://github.com/MalekiRe/stereokit-rs>:
- by abandoning the automatic bindgen to avoid the multiplication of data types and eliminate costly transmutes.
- by implementing all the missing API calls and data types.
- by getting closer to the C# object model.
- by offering a framework based on winit/android-activity which takes up the ISTEPPERS of the C# framework.


This project is at an early stage so try it carefully. Right now, the only way to get the project is to get the source code from github.

### Download the source project:
* git clone https://github.com/mvvvv/StereoKit-rust/
* On linux get the following dev libraries : clang-12 lld-12 ninja-build libx11-dev libxfixes-dev libegl-dev libgbm-dev libfontconfig-dev


### Run the project's demo on your PC's headset :
* Make sure you have [OpenXR installed](https://www.khronos.org/openxr/) with an active runtine.
* Launch: `cargo run  --example main_pc`

### Run the project's demo on your PC using the [simulator](https://stereokit.net/Pages/Guides/Using-The-Simulator.html) 
* Launch: `cargo run  --example main_pc -- --test`

If you're using VsCode you'll see two launchers in launch.json to debug the demo.


### Run the project's demo on your Android headset:

* Install [sdkmanager](https://developer.android.com/studio/command-line/sdkmanager)  (or Android Studio if you intend to use it)
* Check that `adb` is connecting to your headset and set a valid NDK path into ANDROID_NDK_ROOT environment variable.
* Install: `cargo install cargo-apk` (cargo-xbuild has not been tested yet)
* Download: `rustup target add aarch64-linux-android` for most of the existing android headsets.
* Launch: `cargo apk run  --example main`

### Template to create your own project:
* git clone https://github.com/mvvvv/stereokit-template/
* If you don't clone the template project in the same directory than the StereoKit-Rust project, you'll have to modify the path of the Stereokit-rust dependency.


## Troubleshooting
Submit bugs on the [Issues tab](https://github.com/mvvvv/StereoKit-rust/issues), and ask questions in the [Discussions tab](https://github.com/mvvvv/StereoKit-rust/discussions)!

The project <https://github.com/StereoKit/StereoKit/> will give you many useful links (Discord/Twitter/Blog)


