## This project is a rust binding of the excellent [StereoKit](https://StereoKit.net) [project](https://github.com/StereoKit/StereoKit/)

![Screenshot](/StereoKit-rust.png)

<p align="center">
    <a href="https://github.com/mvvvv/StereoKit-rust/branches"><img src="https://img.shields.io/github/last-commit/mvvvv/StereoKit-rust/master" /></a>
    <a href="https://tldrlegal.com/license/mit-license"><img src="https://img.shields.io/github/license/mvvvv/StereoKit-rust" /></a>
    ![Build and test](https://github.com/mvvvv/StereoKit-rust/actions/workflows/rust.yml/badge.svg)
</p>

Its purpose is to improve the previous rust project <https://github.com/MalekiRe/stereokit-rs>:
- by abandoning the automatic bindgen to avoid the multiplication of data types and eliminate costly transmutes.
- by implementing all the missing API calls and data types.
- by getting closer to the C# object model.
- by offering a framework based on winit/android-activity which takes up the ISTEPPERS of the C# framework.


This project is at an early stage so try it carefully. Right now, the only way to get the project is to get the source code from github.

#### Regarding your working OS, here are the target architectures you can build for:

| Target: | Windows x86_64| Linux x86_64       | Meta Quest |  Linux AArch64     | 
|:-------:|:-------------:|:------------------:|:----------:|:------------------:|
| Windows x86_64 | **X**  | with Steam Proton  |     **X**  |  |
| Linux  x86_64  | **X**  |            **X**   |     **X**  |  **X**             |
| Linux  AArch64 |  ?     |         ?          |       ?    |  **X**             |
| MacOs          |        |                    |     **X**  |  |  
Let us know if you have launched the demos on an architecture not tested here.


### Download the source project:
* `git clone --recursive https://github.com/mvvvv/StereoKit-rust/`
* On Linux get the following tools and dev libraries : clang cmake libx11-dev libxfixes-dev libegl-dev libgbm-dev libfontconfig-dev.
* On Windows[^2] get the following tools and dev libraries : "CMake", "Visual Studio Build Tools 2022(Developpment Desktop C++)" and "DotNet SDK v8+" 
* Install the project's tools from the project directory `cargo install -F event-loop --path .`
* If you want to launch the demos then: 
  - compile the shaders. From StereoKit-rust directory launch `cargo compile_sks` 
  - for Windows only and if you don't use VSCode launchers, add to the PATH environment variable the directory `./target/debug/deps`

### Run the project's demo on your PC's headset :
* Make sure you have [OpenXR installed](https://www.khronos.org/openxr/) with an active runtine.
* Launch[^1]: `cargo run --features event-loop  --example main_pc`

### Run the project's demo on your PC using the [simulator](https://stereokit.net/Pages/Guides/Using-The-Simulator.html) 
* Launch[^1]: `cargo run --features event-loop  --example main_pc -- --test`

### Build and create an exportable repository of project's demo for your PC
`cargo build_sk_rs --example main_pc --features event-loop <the path of your exportable repository>`

### Run the project's demo on your Android headset (from a PC running Windows, Mac or Linux):
* Install [sdkmanager](https://developer.android.com/studio/command-line/sdkmanager)  (or Android Studio if you intend to use it). You'll need a Java JDK (v17 is fine).
* Using sdkmanager, install platform-tools(v32), latest build-tools and the latest ndk. 
* Set ANDROID_HOME environment variable to its path (this path contains the `build_tools` directory). 
* Set the NDK path (which ends with it's version number) into ANDROID_NDK_ROOT environment variable.
* Install [Ninja](https://ninja-build.org/) 
* Check that `adb` ($ANDROID_HOME/platform_tools/adb) is connecting to your headset.
* Install: `cargo install cargo-apk` (cargo-xbuild has not been tested yet).
* Download the target: `rustup target add aarch64-linux-android` for most of the existing android headsets.
* Launch: `cargo apk run --features event-loop  --example main`

### Use your own event manager (PC only - see gradle templates for an android build)
The demos above, are using [winit](https://github.com/rust-windowing/winit) as an event manager and interface with the OS. If you want to use your own loop and event manager, have a look to [manual.rs](https://github.com/mvvvv/StereoKit-rust/blob/master/examples/manual.rs).
This is the shortest way to launch your first PCVR/PCMR program[^1]: `cargo run --features no-event-loop --example manual`


## Templates to create your own project:
There is 3 templates used to build android versions (they can also create a PCVR executable). The default choice, branch `main`, will use cargo-apk (like demos above). The branch `gradle` will let you use gradle with winit. Then the branch `gradle-no-event-loop` will use gradle without winit.
* `git clone -b $branch https://github.com/mvvvv/stereokit-template/`
* If you don't clone the template project in the same directory than the StereoKit-rust project, you'll have to modify the path of the Stereokit-rust dependency.


## (Temporary needs some stereokit/sk_gpu hacks on linux) Build the project's demo for Windows_x64 using GNU from Linux (and Windows and probably Mac)
* Install mingw64-w64 (MSYS2 on windows)
* Add the rust target gnu for windows:`rustup target add x86_64-pc-windows-gnu`
* Temporary unavailable for linux: we need wine to compile the shaders
  - Install wine and winetricks.
  - Install needed tools and libs: `winetricks corefonts d3dx9 d3dcompiler_47`
* Create a directory where necessary libs will be stored (i.e. ../x64-mingw-libs/) then add a link to the DLLs or static libs(*.a) the build will need after or during its creation. Example on Ubuntu 24.XX:
  - `ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libgcc_s_seh-1.dll ../x64-mingw-libs/ && ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libstdc++-6.dll ../x64-mingw-libs/`
  - or `ln -s /usr/lib/gcc/x86_64-w64-mingw32/13-win32/libgcc.a ../x64-mingw-libs/libgcc_eh.a`
* Launch: `cargo build_sk_rs --x64-win-gnu ../x64-mingw-libs/  --example main_pc --features event-loop <the path of your exportable repository>`
* To run main_pc.exe on Linux:
  - Add a non-steam game to your library then launch it when WiVRn or SteamVR are started.
  - If you only need the simulator: `wine main_pc.exe --test`

## Build the project's demo for Linux aarch64 from Linux x86_64
* Install g++-aarch64-linux-gnu
* Get the libraries libx11-dev:arm64 libxfixes-dev:arm64 libegl-dev:arm64 libgbm-dev:arm64 libfontconfig-dev:arm64. On Ubuntu 24:XX this can be done by adding a foreign architecture `dpkg --add-architecture arm64` with depot `http://ports.ubuntu.com/ubuntu-ports`. To avoid errors during `apt update` you'll have to precise the architectures of all depots in `/etc/apt/sources.list.d/ubuntu.sources`
* Add the rust target aarch64 for linux:`rustup target add aarch64-unknown-linux-gnu`
* Add a section `[target.aarch64-unknown-linux-gnu]` in your config.toml for setting `linker = "aarch64-linux-gnu-gcc"`
* Launch `cargo build_sk_rs --example main_pc --features event-loop --aarch64-linux <the path of your exportable repository>`

## Build the project's demo for Linux x86_64 from Linux aarch64 (Not Tested)
* The logic should be the same than previous one.
* Launch `cargo build_sk_rs --example main_pc --features event-loop --x64-linux <the path of your exportable repository>`.

## Troubleshooting
Submit bugs on the [Issues tab](https://github.com/mvvvv/StereoKit-rust/issues), and ask questions in the [Discussions tab](https://github.com/mvvvv/StereoKit-rust/discussions)!

The project <https://github.com/StereoKit/StereoKit/> will give you many useful links (Discord/Twitter/Blog).


## Dependencies

This project was made possible thanks to the work of many talents on the following projects:
* [StereoKit](https://github.com/StereoKit/StereoKit/tree/cb6717aa8bc853e039bf3e0751cf4bff24c94910?tab=readme-ov-file#dependencies) which itself is based on valuable projects.
* [rust_mobile](https://github.com/rust-mobile) used for the android specific code.
* [winit](https://github.com/rust-windowing/winit) used for cross-platform management. 
* [openxrs](https://github.com/Ralith/openxrs) nice binding of OpenXR.
* [blender](https://www.blender.org/) for gltf files, HDRI, models and demo animations
* [gimp](https://www.gimp.org/) for icons files.
* bitflags.
* android_logger.
* this_error & anyerror.
* ... many others, more discreet, without which nothing would be possible.

[^1]: If you're using VsCode you'll see a corresponding launcher in launch.json to debug the app.

[^2]: If you're using VsCode you can choose to use LLDB instead of GDB when testing with MSVC. For that add to your workspace settings.json:
      ```"lldb.script": { "lang.rust.toolchain": "stable-x86_64-pc-windows-gnu" }```