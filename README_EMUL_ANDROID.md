# Run `sk_demos` on the Android emulator (Android XR)

This document describes the whole toolchain to build, install and run the `sk_demos` project
(a Rust/StereoKit demo) on the **Android XR emulator** under Linux x86_64.

Reference configuration for this machine:

| Item | Value |
| --- | --- |
| Gradle project | `~/dvlt/sk_demos` (linked to `~/dvlt/StereoKit-rust`) |
| Android SDK (`ANDROID_HOME`) | `~/android` |
| NDK (`ANDROID_NDK_ROOT`) | `~/android/ndk/29.0.14206865` (r27 `27.1.12297006` also works) |
| AVD | `XR4` — Android XR, API 34 image, **x86_64** |
| System image | `system-images;android-34;android-xr-preview-playstore;x86_64` (channel 3) |
| JDK | 17 |
| Gradle | 8.9 (through the wrapper), AGP 8.5.0, `mozilla.rust-android` plugin 0.9.6 |
| Emulator | 37.2.13.0, KVM acceleration available |
| Rust target | `x86_64-linux-android` |

> ⚠️ The most important point: **the emulator is x86_64**, whereas
> `gradle.properties` ships with `cargo.targets=arm64` (meant for a real headset).
> You must therefore set `cargo.targets=x86_64`, otherwise the APK contains no
> `lib/x86_64/libsk_demos.so` and the app crashes on startup
> (`UnsatisfiedLinkError`).

---

## 0. Quick start

```bash
# 1. environment
export ANDROID_HOME=~/android
export ANDROID_NDK_ROOT=~/android/ndk/29.0.14206865
export PATH="$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin"

# 2. XR emulator (Wayland session -> QT_QPA_PLATFORM=xcb is required)
QT_QPA_PLATFORM=xcb emulator -avd XR4 -gpu host &
adb wait-for-device

# 3. emulator target (x86_64, not arm64)
cd ~/dvlt/sk_demos
sed -i 's/^cargo.targets=.*/cargo.targets=x86_64/' gradle.properties

# 4. build + install + launch + logcat
./gradlew run --warning-mode all && sh logcat.cmd
```

---

## 1. Prerequisites

### 1.1 Android SDK packages

All the commands below use the `sdkmanager` from `~/android/cmdline-tools/latest/bin`

```bash
SDK=~/android/cmdline-tools/latest/bin
export ANDROID_HOME=~/android

$SDK/sdkmanager --update
$SDK/sdkmanager "platform-tools" "platforms;android-35"     # + platforms;android-33 (the project's compileSdk)
$SDK/sdkmanager --install "build-tools;35.0.0"
$SDK/sdkmanager --install "ndk;29.0.14206865"               # or "ndk;27.1.12297006"
$SDK/sdkmanager emulator
```

### 1.2 Android XR system image (channel 3)

XR images live on **channel 3 (canary/preview)**; they do not show up in the default
list:

```bash
$SDK/sdkmanager --list --channel=3 | grep -E "emulator|google-xr|android-xr"
$SDK/sdkmanager --channel=3 "system-images;android-34;android-xr-preview-playstore;x86_64"
```

Images for Android XR (chanel 3):

* `~/android/system-images/android-34/android-xr-preview-playstore/x86_64` <- **the one we need for avd `XR4`**
* `~/android/system-images/android-34/android-xr-v3-playstore/x86_64`
* `~/android/system-images/android-34/google-xr/x86_64`

Only the **x86_64** images matter here: ARM images do not boot (or boot very
slowly, without acceleration) on an x86_64 host.

### 1.3 Host tools

```bash
java -version          # openjdk 17 (verified: 17.0.20.1)
ninja --version        # required by the CMake build of StereoKitC
cmake --version
rustc -V && cargo -V   # 1.99.1 here
rustup target add x86_64-linux-android     # the emulator target
# rustup target add aarch64-linux-android  # only for a real headset
```

### 1.4 Hardware acceleration (KVM)

The XR emulator is unusable without acceleration (both the Vulkan rendering and
StereoKitC's AVX instructions depend on it):

```bash
ls -l /dev/kvm                 # crw-rw----+ 1 root kvm ...
$ANDROID_HOME/emulator/emulator -accel-check
# -> accel: 0  KVM (version 12) is installed and usable.
```

If `/dev/kvm` is missing: enable virtualization in the BIOS and add the user to the
`kvm` group (`sudo usermod -aG kvm $USER`, then log back in).

---

## 2. Create the `XR4` AVD

### Option A — Android Studio (recommended for the "XR Glasses" form factor)

`Device Manager` → `Add a new device` → `XR` / `XR Glasses` → `Android XR` API 34 image
(`android-xr-preview-playstore`, x86_64).
The `xr_glasses_device` device profile (used by `XR4`) is provided by Android
Studio; it **does not exist** in `avdmanager list device` for an SDK installed
from the command line only.

### Option B — command line, then adjust `config.ini`

```bash
$SDK/avdmanager create avd -n XR4 -k "system-images;android-34;android-xr-preview-playstore;x86_64"
```

Then align `~/.android/avd/XR4.avd/config.ini` with the XR values
(excerpt from this machine's file, the essential keys):

```ini
Target = android-xr [Android XR] (API level 34)
target = android-34
tag.id = android-xr
tag.ids = android-xr,google_apis_playstore
image.sysdir.1 = system-images/android-34/android-xr-preview-playstore/x86_64/

hw.cpu.arch = x86_64
hw.cpu.ncore = 4
hw.ramSize = 8192
disk.dataPartition.size = 6G

Skin = 1920x1200
skin.dynamic = yes
hw.lcd.width = 1920
hw.lcd.height = 1200
hw.lcd.density = 320
hw.initialOrientation = landscape

hw.gpu.enabled = yes
hw.gpu.mode = host
hw.gpu.vram = 4096
feature.Vulkan = on
feature.VulkanNativePassthrough = on
```

Checks:

```bash
$ANDROID_HOME/emulator/emulator -list-avds        # -> XR4
cat ~/.android/avd/XR4.ini                        # path=..., target=android-34
```

## 3. Start the emulator

```bash
export ANDROID_HOME=~/android
export PATH="$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator"

# Wayland: QT_QPA_PLATFORM=xcb is required (otherwise the window never opens)
QT_QPA_PLATFORM=xcb emulator -avd XR4 -gpu host &
```

Useful options:

| Option | Effect |
| --- | --- |
| `-gpu host` | **required**: native Vulkan rendering (the `Vulkan`/`VulkanNativePassthrough` features of `config.ini`) |
| `-no-snapshot-load` | cold boot, to be used when a snapshot is corrupted |
| `-no-boot-anim` | faster boot |
| `-wipe-data` | resets the user data (last resort) |
| `-no-window` | **useless here**: the window is needed to see the XR scene |

---

## 4. Configure the `sk_demos` project

### 4.1 `~/dvlt/sk_demos/gradle.properties`

| Key | Value for the emulator | Purpose |
| --- | --- | --- |
| `cargo.targets` | **`x86_64`** (instead of `arm64`) | the `rust-android` plugin builds `x86_64-linux-android` and `app/build.gradle` maps it to the `x86_64` ABI (+ copies `libc++_shared.so` from the NDK) |
| `cargo.libname` | `sk_demos` | name of the `cdylib` → `libsk_demos.so` |
| `cargo.openxr` | `build-dynamic-openxr` | builds the Khronos loader `libopenxr_loader.so` and copies it into `target/runtime_libs/x86_64/` (bundled in the APK) |
| `android.compileSdkVersion` | `33` | ⇒ the `platforms/android-33` directory must be installed |
| `android.minSdkVersion` | `29` | fine with the API 34 image |
| `android.applicationId` | `com.stereokit.sk_demos` | used by `adb shell am start` |
| `android.main` | `com.stereokit.sk_demos.MainActivity` | Java activity calling `System.loadLibrary("openxr_loader")` then `System.loadLibrary("sk_demos")` |
| `adb.portFrom` / `adb.portTo` | `tcp:8080` / `tcp:8080` | used by the `reversePort` task (`adb reverse`) |

```bash
cd ~/dvlt/sk_demos
sed -i 's/^cargo.targets=.*/cargo.targets=x86_64/' gradle.properties
grep -n "cargo.targets" gradle.properties       # cargo.targets=x86_64
```

Alternative without editing the file: `./gradlew -Pcargo.targets=x86_64 run`
(a property passed with `-P` takes precedence over `gradle.properties`; this
override is what was used to validate the configuration above).

`cargo.openxr=use-that-of-the-device` is the alternative if you prefer to use the
loader shipped by the system image; `build-dynamic-openxr` is the safest choice
(same behaviour as on a headset) and is the current value.

### 4.2 Project links (reminder)

The `sk_demos` project holds no code: everything is linked to the
`~/dvlt/StereoKit-rust` repository (see the "Run the project's demo on your Android
headset" section of the repository's [`README.md`](README.md)):

## 5. Build, install and run

### 5.1 The all-in-one `run` task

```bash
cd ~/dvlt/sk_demos
./gradlew run --warning-mode all
```

What `run` does (defined in `~/dvlt/sk_demos/build.gradle`):

1. `:app:cargoBuild` — CMake builds StereoKitC (OpenXR loader included) for the
   `x86_64` ABI, then `rustc` produces
   `target/x86_64-linux-android/debug/libsk_demos.so` and `build.rs` copies
   `libopenxr_loader.so` + `sk_app.jar` into `target/runtime_libs/`;
2. `:app:assembleDebug` — the APK is written to
   `app/build/outputs/apk/debug/app-debug.apk`;
3. `:app:installDebug` — installs it on `emulator-5554`;
4. `launchDebug` — `adb shell am start -a android.intent.action.MAIN -n com.stereokit.sk_demos/com.stereokit.sk_demos.MainActivity`;
5. `getUid` — rewrites `logcat.cmd` with the real UID of the app
   (`adb logcat -v color --uid <uid>`).

The first build is long (full compilation of StereoKitC + the OpenXR loader for
this ABI, several minutes); the next ones are incremental.

### 5.2 Useful variants

```bash
./gradlew buildDebug            # APK only
./gradlew :app:installDebug     # install only
./gradlew launchDebug           # install + am start
./gradlew reversePort           # adb reverse tcp:8080 tcp:8080 (optional)
```

Manual install/launch:

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -a android.intent.action.MAIN \
  -n com.stereokit.sk_demos/com.stereokit.sk_demos.MainActivity
```

> **Changing ABI** (arm64 ↔ x86_64): run `./gradlew clean` (at least
> `:app:clean`) before rebuilding. Otherwise the stale `.so` files of the other ABI
> stay in `app/build/intermediates` and the APK becomes a misleading "mixed" APK
> (observed: both `lib/arm64-v8a/` **and** `lib/x86_64/` in the same APK).

---

## 6. Check that the app is running

```bash
adb devices
adb shell pm list packages | grep sk_demos
sh logcat.cmd                      # adb logcat -v color --uid <uid>
adb exec-out screencap -p > /tmp/sk_demos.png
```

In the logs you should see (tags `StereoKitJ` and `STKit-rs`):

```text
D/StereoKitJ: !!!!onCreate
... Runtime <runtime> / Device <name> / Emulator Some(Android)
```

The `Emulator Some(Android)` line is produced by `Device::is_emulator()`
(`src/util.rs`), which detects `ro.hardware = ranchu|goldfish`,
`ro.product.device = emulator*`, etc. It confirms that StereoKit really got an
OpenXR session on the emulator (and not a 2D fallback).

The app then shows the **Demos** window (`Test` buttons), the log window, the
`ControlPanel` and the `Press [F1] key to open the hand menu` notification.

---

## 7. Interacting with the emulator

* **Mouse**: gaze / pointer; **left click** = primary action (select). hold `Alt` to rotate the camera.
* **Move around**: hold `Alt`, then `W`/`A`/`S`/`D` (or the arrow keys), `Q` down,
  `E` up (on macOS it is `Option`).
* **`F1`**: doesn't open the `sk_demos` radial *hand menu* (the app detects the emulator
  and shows that hint). You have to inject F1 from the main window.
* Emulator top menu: `Toggle Passthrough`, screenshot / recording, Power / Volume /
  Back / Home / Overview buttons.
* **Known limitations of the XR emulator**: only the *right-hand pinch* is simulated
  (no grab, no poke) — see the `Emulator` documentation in `src/util.rs`.
  `Device::is_emulator()` should therefore be used as a hint, not as a substitute
  for a capability check (`Device::has_hand_tracking()`).

To go back to a real headset: `cargo.targets=arm64`,
`rustup target add aarch64-linux-android`, then `./gradlew clean && ./gradlew run`.

---

## 8. Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| `GradleException: ANDROID_NDK_ROOT environment variable is not set.` | `export ANDROID_NDK_ROOT=~/android/ndk/29.0.14206865` (the directory name must be the NDK version) |
| `Failed to find target with hash string 'android-33'` / `compileSdk 33` missing | `$SDK/sdkmanager "platforms;android-33"` |
| The app closes immediately, `UnsatisfiedLinkError: dlopen failed: library "libsk_demos.so" not found` | the APK has no x86_64 ABI: check `cargo.targets=x86_64`, then `./gradlew clean && ./gradlew run` |
| `libopenxr_loader.so not found` | `cargo.openxr=build-dynamic-openxr` must be active; check `target/runtime_libs/x86_64/libopenxr_loader.so` after the build |
| APK containing both `arm64-v8a` **and** `x86_64` | build leftovers: `./gradlew clean && rm -rf app/build`, then rebuild |
| `adb devices` empty | `adb kill-server && adb start-server && adb devices`; check that the emulator window is open |
| The emulator does not start / black screen | restart with `-no-snapshot-load`, then `-wipe-data` as a last resort; check `emulator -accel-check` |
| The Qt window cannot open (Wayland session) | prefix the command with `QT_QPA_PLATFORM=xcb` |
| `Illegal instruction (SIGILL)` at launch | StereoKitC x86_64 is compiled with `-mf16c -mavx` (`build.rs`): the emulator must run with KVM on an x86_64 host that supports AVX (no ARM host, no nested virtualization) |
| `avdmanager list avd`: `Failed to parse properties ... config.ini` | cosmetic (spaced `Key = value` keys); the emulator still lists `XR4` — see §2 |
| Very long build / full disk | `target/` and `app/build/` are big (StereoKitC per ABI); use `./gradlew clean` and `cargo clean` between test runs |
| The app shows up in 2D (not immersive) | check the AVD's `tag.id = android-xr` / `image.sysdir.1` and the manifest (`android.window.PROPERTY_ACTIVITY_XR_START_MODE=FULL_SPACE_UNMANAGED`, `IMMERSIVE_HMD` category) |

---

## 9. What was verified

```text
~/android/emulator/emulator -list-avds          -> XR4
~/android/emulator/emulator -accel-check        -> KVM (version 12) installed and usable
ls -l /dev/kvm                                  -> crw-rw----+ root kvm
rustup target list --installed                  -> ... x86_64-linux-android ...
ls ~/android/platforms                          -> android-32 33 34 35 36.1
ls ~/android/ndk                                -> 27.1.12297006  29.0.14206865
cd ~/dvlt/sk_demos && ./gradlew -Pcargo.targets=x86_64 --offline help   -> BUILD SUCCESSFUL
```

The Gradle configuration (`ANDROID_NDK_ROOT`, `compileSdk 33`, the `rust-android`
plugin, `cargo.targets` overridden to `x86_64`) is therefore validated; only
starting the emulator is left before `./gradlew run` installs and launches the app.

---

## 10. References

* `~/dvlt/StereoKit-rust/README.md` — "Run the project's demo on your Android headset" section.
* `~/android/sdkmanager_histo.txt` — log of the SDK packages installed.
* `~/.android/avd/XR4.avd/config.ini` — XR Glasses AVD configuration.
* Android XR — *Run your app's immersive experiences on the emulator*:
  <https://developer.android.com/develop/xr/jetpack-xr-sdk/run/emulator/xr-headsets-glasses>
* Android XR — *Run and debug your immersive app experiences*:
  <https://developer.android.com/develop/xr/jetpack-xr-sdk/studio-tools>
* `sdkmanager`: <https://developer.android.com/studio/command-line/sdkmanager>
* `Device::is_emulator()` / `Emulator`: `StereoKit-rust/src/util.rs`.
