# Hot reloading: the `cargo run_sk` viewer

`cargo-run_sk` is a real-time viewer for a StereoKit-rust project under development. It keeps **one** StereoKit
session alive (Simulator by default, real OpenXR with `--xr`, headless with `--offscreen`) and hot-reloads the
plugin library of the project each time it is rebuilt: the new library is loaded **inside the running process**,
without ever closing the session or re-pairing a headset. The views of the project (its `Test`s / `IStepper`s) are
offered in a selector window, with on-demand screenshots.

```text
        cargo-run_sk (host)                      plugin = the project (cdylib)
  +---------------------------+          +-------------------------------------+
  | ONE SkSettings, ONE       |          | sk_run_sk_settings() -> SkSettings  |
  | Sk session, ONE engine    |  dlopen  | sk_run_sk_views_count() / _view_info|
  | (Simulator / XR / Offscr) | <------> | sk_run_sk_begin(sk_info)            |
  | Steppers: HotReloading +  | 100% C   | sk_run_sk_select(index)             |
  |           LogWindow       |  ABI     | sk_run_sk_step(sk_info, token)      |
  +---------------------------+          | sk_run_sk_end()                     |
              |                          +-------------------------------------+
              |                                          |
              +------------- libStereoKitC.so -----------+   (same instance, one engine)
```

---

## 1. Architecture

The retained design ("Option B") is an **in-process plugin with a real, continuous StereoKit session**:

- The host is a normal StereoKit-rust binary: a single `SkSettings`, a single `Sk` session, and a single engine.
- The "dynamic library of the project" is a **cdylib plugin that does not link its own copy of the native
  libraries** (StereoKitC, sk_renderer, sk_app, openxr_loader...). Both sides link the same shared StereoKitC
  (`libStereoKitC.so` on Linux, `StereoKitC.dll` on Windows, `libStereoKitC.dylib` on macOS, feature `skc-shared`):
  the dynamic loader deduplicates it by SONAME / install name / module name, so the whole process drives **one**
  engine and **one** session. Adding or removing a view is then `end()` + `dlclose`/`FreeLibrary` of the old plugin,
  and `dlopen`/`LoadLibrary` of a new version, without leaving the session.
- The boundary between the host and the plugin is **100% C**: opaque pointers and `#[repr(C)]` structures only (see
  [`src/plugin_abi.rs`](src/plugin_abi.rs) and `crate::plugin_abi`). No Rust type (no `TypeId`, no vtable, no
  generics) ever crosses it.
- The "Test" views stay the existing `IStepper`s (the `examples/demos/*`). They are driven by a **plugin-side**
  `Steppers` manager: the host only initializes the session and swaps plugins, the steppers and views live on the
  plugin side (one copy active at a time).
- The session settings are the settings of the project itself: the plugin exposes them through
  `sk_run_sk_settings` (`sk_settings()`), and the host reads them **before** `Sk::init`.

## 2. Files and components

| Path | Role |
| --- | --- |
| [`Cargo.toml`](Cargo.toml) | `skc-shared = ["dep:libloading"]` feature, `[[bin]] cargo-run_sk` (`required-features = ["skc-shared"]`), `libloading` for `dlopen`/`dlsym` (Linux, macOS) and `LoadLibrary` (Windows). |
| [`build.rs`](build.rs) | `SK_BUILD_SHARED_LIBS=ON` with `skc-shared` (Windows, Linux, macOS) and `SK_DYNAMIC_OPENXR=ON` on Linux/macOS (shared OpenXR loader); links `dylib=StereoKitC` + `dylib=openxr_loader`, copies `libStereoKitC.*` under `target/debug/deps/` for [`cargo-build_sk_rs`](src/bin/cargo-build_sk_rs.rs), embeds an rpath so nothing has to be installed. Elsewhere: warning + static build. |
| [`src/plugin_abi.rs`](src/plugin_abi.rs) | The C contract shared by both sides: `SK_RUN_SK_ABI_VERSION`, `SK_RUN_SK_NAME_MAX`, `PluginViewInfo`, `SkSettingsFn`, the `sk_*` symbol documentation. |
| [`src/tools/hot_reloading.rs`](src/tools/hot_reloading.rs) | The reusable host-side tool: `HotReloading`, an `IStepper` that watches, builds, loads, swaps, selects and captures. Also `plugin_file`, `default_lib_path`, `default_build_cmd`, `default_watch_roots`. |
| [`src/bin/cargo-run_sk.rs`](src/bin/cargo-run_sk.rs) | The `cargo run_sk` viewer: command line parsing, session initialization with the project settings, `HotReloading` + `LogWindow` steppers, main loop. |
| [`src/tools/log_window.rs`](src/tools/log_window.rs) | The log window shown next to the selector window (build tails, plugin messages, StereoKit logs). |
| [`examples/run_sk_plugin/mod.rs`](examples/run_sk_plugin/mod.rs) | The plugin side of this repository, wired in [`examples/main.rs`](examples/main.rs) under `#[cfg(feature = "skc-shared")]`; exposes the `Test::get_tests()` views. |
| [`src/templates/plugin_shim.rs`](src/templates/plugin_shim.rs) | The plugin shim generated for projects created by [`cargo new_sk_rs_project`](src/bin/cargo-new_sk_rs_project.rs) (non-basic projects, `${SK_VERSION}` substituted), declared in [`src/templates/lib_rs_framework.rs`](src/templates/lib_rs_framework.rs) as `#[cfg(all(feature = "skc-shared", not(target_os = "android")))] pub mod plugin_shim;`. |
| [`src/tools/mod.rs`](src/tools/mod.rs) | `pub mod hot_reloading;` under `#[cfg(all(feature = "skc-shared", not(feature = "no-event-loop")))]`. |
| [`src/framework/event_loop.rs`](src/framework/event_loop.rs) | `Steppers::step` and `Steppers::step_post_app` are public, so the copy of the framework **inside the plugin** can run its own steppers without owning the `Sk`/`SkClosures` of the host. |
| [`README.md`](README.md) | User-facing section "Develop with hot-reload, the `cargo run_sk` viewer". |

A new project also gets a `skc-shared = ["stereokit-rust/skc-shared"]` forwarding feature in its generated
`Cargo.toml`, and a "Next steps" line pointing at `cargo run_sk`.

## 3. The plugin ABI

Every symbol below is a `#[no_mangle] extern "C"` function **exported by the plugin** and resolved by the host with
`libloading` right after loading it. Status codes are `0` = success.

| Symbol | Role | Status codes |
| --- | --- | --- |
| `sk_run_sk_version() -> u32` | ABI version guard: must equal `SK_RUN_SK_ABI_VERSION`. | — |
| `sk_run_sk_crate_version() -> *const c_char` | `stereokit-rust` version the plugin was compiled against; the host compares it with its own `env!("CARGO_PKG_VERSION")`. | — |
| `sk_run_sk_settings(*mut SkSettings) -> u32` | Fills the out-parameter with the settings of the project (`sk_settings()`), read **before** `Sk::init`. | 1 null pointer |
| `sk_run_sk_views_count() -> u32` | Number of views exposed. | — |
| `sk_run_sk_view_info(index: u32, *mut PluginViewInfo) -> u32` | Name (nul-terminated, 63 chars max) + `has_screenshot` flag of one view. | 1 null pointer, 2 out of bounds |
| `sk_run_sk_begin(sk_info: *mut c_void) -> u32` | Stores the host session and prepares the plugin-side `Steppers`. `sk_info` points to the host `Rc<RefCell<SkInfo>>` (ABI v2). | 1 null pointer, 2 already begun |
| `sk_run_sk_select(index: u32) -> u32` | Swaps the active view (the previous one is removed properly). | 1 not begun, 2 out of bounds |
| `sk_run_sk_step(_sk_info: *mut c_void, token: *mut c_void) -> u32` | Called once per frame by the host, right after its own app callback: runs the plugin steppers (pre-app then post-app). A view asking to quit closes the plugin views but leaves the host session alive. | 1 null token, 2 not begun |
| `sk_run_sk_end() -> u32` | Shuts the plugin-side steppers down; called before `dlclose`/`FreeLibrary` and at application shutdown. | — |

Notes:

- `SK_RUN_SK_ABI_VERSION` history: `1` for the first version (`begin` received a pointer to the host `Sk`), `2`
  (current) for `begin` receiving a pointer to the `Rc<RefCell<SkInfo>>` of the host session.
- `PluginViewInfo` is the only structure that crosses the boundary (`#[repr(C)]`, explicit padding,
  `SK_RUN_SK_NAME_MAX` = 64 bytes).
- Before calling `begin`, the host checks the ABI version, the crate version and the presence of every symbol. A
  mismatch is reported as a load error: **never mix a host and a plugin built from different versions, features or
  toolchains of `stereokit-rust`**.

## 4. Using the viewer

### 4.1 Installation

```shell
# from the project directory (installs cargo-compile_sks, cargo-build_sk_rs, cargo-new_sk_rs_project, cargo-run_sk)
cargo install --path . -F skc-shared

# or from crates.io
cargo install stereokit-rust -F skc-shared
```

Once installed, `cargo run_sk` (cargo resolves the unknown subcommand to the `cargo-run_sk` binary) can be used from
any StereoKit-rust project.

### 4.2 This repository

```shell
# Simulator, fully automatic: watches src/, examples/, assets/, shaders_src/, Cargo.toml, build.rs,
# builds the plugin (cargo build --example main --features skc-shared) at startup and on each change.
cargo run --bin cargo-run_sk --features skc-shared

# real OpenXR runtime, here the Monado simulator
XR_RUNTIME_JSON=/usr/share/openxr/1/openxr_monado.json cargo run --bin cargo-run_sk --features skc-shared -- --xr

# headless: run 120 frames on one view, screenshot it, quit
cargo run --bin cargo-run_sk --features skc-shared -- --offscreen --test 120 --start Ui1

# print the views of the (already built) plugin and exit, without any StereoKit session
cargo run --bin cargo-run_sk --features skc-shared -- --list
```

### 4.3 Your own project

Projects created with `cargo new_sk_rs_project` (non-basic) ship with `src/plugin_shim.rs`, where the views are
declared:

```shell
cd my_project
cargo install stereokit-rust -F skc-shared   # if not already done
cargo run_sk
# then edit src/ : the viewer rebuilds and reloads your views on the fly
```

`src/plugin_shim.rs` holds the list of views (`views()`), each one a name + a factory returning the
`StepperAction::add_default::<MyStepper>("My view")` to run, and forwards `crate::sk_settings()` to the host through
`sk_run_sk_settings`. Add one line per view; the `(img)` flag is deduced from the conventional screenshot path
`screenshots/<name lowercased, spaces as underscores>.jpeg`.

### 4.4 Command line

| Option | Effect |
| --- | --- |
| `--simulator` | Start in Simulator mode (default). |
| `--xr` | Start in OpenXR mode, no flatscreen fallback. |
| `--offscreen` | Start without any display (CI, screenshots). |
| `--fullscreen` | Ask the desktop window to start fullscreen. |
| `--lib <path>` | Compiled plugin library to load. A `.rs` path is rejected with a hint: this is the compiled artifact, not the source. |
| `--start <view>` | View selected at start **and after each reload** (exact name, case-insensitive name, or substring). |
| `--watch <secs>` | Plugin library watch period in seconds, `0` disables it (default `0.5`). |
| `--build-cmd <command>` | Build command run at startup and on each source change (run through `sh -c` / `cmd /C`). |
| `--no-build` | Don't watch the sources and don't auto-build: a build made by hand is still picked up. |
| `--test [N steps]` | Run `N` frames (default 1000), screenshot the active view, then quit. |
| `--list` | Print the views of the plugin and exit (no StereoKit session). |
| `-h`, `--help` | Display the usage. |

Default plugin library (`default_lib_path`), guessed from the sources present (not from build artifacts):

1. `src/bin/main_<crate>.rs` exists → a `cargo new_sk_rs_project` project, plugin `target/debug/<plugin of the crate>`;
2. else `examples/main*.rs` exists → this repository, plugin `target/debug/examples/<plugin of the example>`
   (`libmain.so` / `main.dll` / `libmain.dylib`);
3. else the first plugin file under `target/debug/examples/`.

Default build command (`default_build_cmd`): `cargo build --example <name> --features skc-shared` in this repository,
`cargo build --lib --features skc-shared` otherwise (`--lib` never touches the running executable, which cannot be
replaced on Windows). The default watch roots (`default_watch_roots`) are the ones that exist among `src`,
`examples`, `assets`, `shaders_src`, `Cargo.toml`, `build.rs`.

If the plugin was last built **without** the `skc-shared` feature (a plain `cargo build --example main`, for example),
its library exports no `sk_run_sk_*` symbol at all and the load fails with
`plugin has no symbol sk_run_sk_version: dlsym failed`: the default build command above rebuilds it with the right
feature.

## 5. Library API of the tool

`HotReloading` is a plain `IStepper`, so it can be embedded in any StereoKit-rust application, not only in the
`cargo run_sk` viewer:

```text
let mut hot_reloading = HotReloading::auto_detect();
let mut settings = SkSettings::default();
if let Err(err) = hot_reloading.apply_plugin_settings(&mut settings) {
    Log::warn(format!("hot_reloading: {err}"));
}
let mut sk = settings.init().expect("cannot initialize StereoKit");
sk.send_event(StepperAction::add("hot_reloading", hot_reloading));
SkClosures::new(sk, |_sk, _token| {}).run();
```

Constructors and builders (all consuming `self` for chaining):

| Member | Effect |
| --- | --- |
| `HotReloading::new(lib_path)` | Tool for the COMPILED plugin library at `lib_path`; auto-build disabled until `build_cmd` is set. |
| `HotReloading::auto_detect()` | Tool for the project of the current directory (`default_lib_path` + `default_build_cmd`). |
| `.build_cmd(command)` | Build command run at startup then on each source change; build errors are never fatal. |
| `.no_build()` | Disable the auto-build (no source watch, no command); the library is still watched. |
| `.watch(secs)` | Plugin library watch period, `0` disables it (default `0.5`). |
| `.watch_roots(roots)` | Files/directories scanned for source changes (default `default_watch_roots()`). |
| `.start_view(name)` | View to select at start and after each reload (exact, case-insensitive, or substring). |
| `.test_steps(steps)` | Test mode: screenshot `screenshots/run_sk_<view>.jpeg` after `steps` frames, then quit. `0` disables it. |
| `.show_ui(show)` | Show or hide the selector window (shown by default); hiding it is useful for a test run. |
| `.appearence(appearence)` | Replace the look of the selector window (size, scale, text styles, tints...). |
| `.window_pose(pose)` | Pose of the selector window. |
| `apply_plugin_settings(&mut settings)` | Load the plugin **without a session**, read its settings through `sk_run_sk_settings`, keep it preloaded (begun at the first frame). Returns `Err` without touching `settings` when the plugin is not loadable yet: a warning is enough, the session starts with the host settings. |
| `HotReloading::list_views(lib_path)` | Load the plugin without a session and return the `(name, has_screenshot)` of its views. |

Free functions: `plugin_file(name)` (`lib<name>.so`, `<name>.dll`, `lib<name>.dylib`), `default_watch_roots()`,
`default_lib_path()`, `default_build_cmd(lib_path)`.

## 6. How a reload works

### 6.1 Before `Sk::init`

`apply_plugin_settings` loads the plugin once **without any session**, calls `sk_run_sk_settings` and keeps the
loaded library as the `preloaded` plugin. The host then applies its own launch-dependent settings on top
(`app_name`, `fullscreen`, `mode`) and initializes StereoKit. A plugin that is not loadable yet is only a warning:
the session starts with the host settings.

### 6.2 Each frame

`HotReloading::step` (stepper priority `+1`, so it runs **after** the app callback) does, in order:

1. drain the messages of the background watchers (`LibChanged`, `BuildStarted`, `BuildFinished`);
2. unload the "zombie" library of a plugin ended on the previous frame;
3. (re)load the plugin if needed;
4. draw the selector window, apply a pressed view / a capture;
5. forward the frame to the plugin (`sk_run_sk_step(sk_info_ptr, token_ptr)`);
6. run the test mode.

`shutdown` calls `sk_run_sk_end` and drops the preloaded/zombie/watcher state.

### 6.3 Background watchers

- **Plugin library** (`watch_lib`): every `--watch` seconds (default `0.5`), a fingerprint of the library
  (modification time + size + hash of the first 4 KiB) is compared with the previous one. A `LibChanged` is sent
  only when two consecutive samples are identical, because cargo may still be writing the file on the first one.
- **Sources and build** (`watch_sources_and_build`, only when a build command is set and there are watch roots):
  the sources are scanned every 400 ms (sum of the modification times under the roots, depth ≤ 6). When the sum
  changes, the scan repeats until it is quiet (`1 s` debounce, an edit or a `cargo` run touches many files), then
  `run_build` executes the command through `sh -c` / `cmd /C`. The build outcome is reported to the main loop; on
  failure the last 8 stderr lines are shown in the selector window. The build is also run once at startup, so a
  fresh project is built before the viewer needs its plugin.

### 6.4 Safe swap

The (re)load itself is driven by the main thread, between two frames:

1. the new library is **copied** to `target/run_sk/plugin-<pid>-<counter>.<ext>` and this copy is loaded: a loaded
   DLL cannot be overwritten on Windows, and the copy lets cargo replace the original file while the previous
   plugin stays loaded (stale copies of previous sessions are cleaned up);
2. ABI version, crate version and every symbol are checked, the view list is read, then `sk_run_sk_begin` is called;
3. **only then** is the previous plugin `end()`ed and its library dropped one frame later (the "zombie" step), so
   its steppers can finish cleanly;
4. the previously active view is re-selected **by name** in the new plugin (fallback `--start`, else none). Matching
   is exact name → case-insensitive name → substring.

Failure handling: a load failure (broken build, ABI mismatch, missing symbol) logs an error, keeps the **last
working plugin loaded** and does not retry that exact file on every frame — only when it changes or when `Reload` is
pressed. A failed build never kills the session either.

### 6.5 Test mode and screenshots

- `--test N` (`test_steps`): the first view is selected if none is active, after `N` frames the active view is
  screenshotted as `screenshots/run_sk_<view>.jpeg` (view name lowercased, spaces and slashes as underscores), then
  the app is asked to quit one frame later through a `StepperAction::Quit`, so the shutdown sequence stays the
  normal one.
- The `Capture` button of the selector window takes the same screenshot on demand.
- The screenshot viewpoint is the one of the demos test mode, in front of the views laid out around the origin.

## 7. Selector window and log window

The `run_sk` window (a `HotReloading` stepper) shows:

- the **fps**, smoothed over one second with an exponential moving average (`smooth_fps`), so the value is not
  frame-rate dependent;
- a **status line**: `n views, active: <name>`, `building...`, `build ok`, `build FAILED:` + the stderr tail,
  `load failed: <reason>`, `waiting for plugin`;
- the forced **`Reload`** (reload the current library, bypassing the file change detection) and **`Capture`**
  buttons;
- the **view list** as a grid: one button per view, as many columns as the window width allows, the active view is
  tinted, `(img)` is appended when a screenshot file already exists, and long names are ellipsized. Pressing a view
  replaces the active one (the previous one is removed by the plugin through `sk_run_sk_select`).

The window goes through `Appearence` (size, ui scale, text styles, tints) and has a grab-able handle after the
window: drag it along the window local X to widen the grid, along Z to scale everything (the height always fits the
grid).

The viewer also adds a `LogWindow` stepper (`tools::log_window`), fed by a log buffer subscribed with `Log::subscribe`
**before** the session starts: StereoKit initialization logs, the build tails, the view selections and the plugin
messages are visible in the session without leaving the viewer. In the `cargo-run_sk` binary the selector window and
the log window are both placed on the left of the session.

## 8. Design decisions and risks

| Decision | Rationale |
| --- | --- |
| **In-process plugin, one continuous session** ("Option B") | Keeps the Simulator/OpenXR session, the assets and the headset pairing alive; only the plugin is swapped. |
| **A cdylib plugin that does not link the engine** | A single engine instance per process; the plugin's unresolved `sk_*` symbols are resolved toward the host engine through the shared StereoKitC (`DT_NEEDED` / SONAME dedup). |
| **Shared StereoKitC (`skc-shared`) instead of `-rdynamic`** | Makes both host and plugin self-contained and portable; the `bindings-only` + `-Wl,-rdynamic` fallback turned out to be unnecessary (kept as a documented track only). |
| **A shared OpenXR loader on Linux/macOS** (`SK_DYNAMIC_OPENXR=ON`) | `openxr-sys` calls `xr*` functions directly from Rust: a second loader folded inside StereoKitC would break cross-boundary `XrInstance` handles. With a shared `libopenxr_loader` both sides share one loader. |
| **100% C boundary** (`plugin_abi`) | Two dylibs of the same crate do not share a stable Rust ABI (vtables, `TypeId`, generics). Only opaque pointers and `#[repr(C)]` structures cross. |
| **Version guards at load time** | The ABI version **and** the `stereokit-rust` version of the plugin must match the host exactly (same features, same toolchain): a mismatch would silently corrupt the structs. |
| **Plugin-side `Steppers`** | The host only initializes and swaps; the views live in the plugin copy of the framework (one copy active at a time), so `sk_run_sk_end` cleans everything before each `dlclose`. |
| **Load the new plugin before retiring the old one** | A broken or incompatible build never kills the running session: the previous plugin keeps driving it. |
| **`end()` first, deferred `dlclose`** | A Rust dylib may have statics and destructors; it is only unloaded on the main thread, one frame after its steppers were shut down (zombie step), like the usual precautions of dynamic Rust plugins. |
| **Copy each loaded library under `target/run_sk/`** | Windows cannot overwrite or delete a loaded DLL: loading a fresh copy lets cargo rewrite the original file. |
| **Re-select the active view by name after a reload** | The plugin state is recreated from scratch (documented, accepted in v1): the view restarts, but the developer keeps looking at the same one. |

## 9. Known limitations

- **Same versions on both sides**: host and plugin must be built from the same `stereokit-rust` version, the same
  features (notably `skc-shared`) and the same toolchain; the host rejects anything else with a clear error.
- **Platforms**: `skc-shared` is implemented for Windows, Linux and macOS. Elsewhere (e.g. Android) the feature is
  ignored with a warning and StereoKitC stays static, so there is no hot-reload; `plugin_shim` is excluded from the
  Android builds. Android remains a supported target for the normal demos.
- **View state is reset on each reload**: the active view is re-selected automatically, but its internal state does
  not survive a swap.
- **The swap happens between two frames**, on the main thread: a small hiccup during a reload, never a mid-frame
  change.
- **View names are limited to 63 characters** (`SK_RUN_SK_NAME_MAX`).
- **A plugin whose load failed is not retried on every frame** (to avoid log spam): only when its file changes or on
  an explicit `Reload`.
- **Both sides must be rebuilt** when `stereokit-rust` or the native libraries change: there is one shared engine,
  but no installed runtime.

## 10. Validation performed

- `cargo check` with the default features: no regression.
- `cargo run --bin cargo-run_sk --features skc-shared -- --list`: 35 views of the `main` plugin listed without any
  StereoKit session.
- `--offscreen --test 20 --start "Test A"`: exit code 0 and a rendered screenshot file.
- Real edit → auto-build → hot swap cycle: `begin/end/begin` traces, automatic re-selection of the active view, the
  session stays alive.
- The plugin resolves its `sk_*` symbols through `DT_NEEDED libStereoKitC.so`, the host loads the same instance
  (SONAME dedup) and `openxr_loader` stays unique.
- Unit tests: `smooth_fps` (in `src/tools/hot_reloading.rs`) and the `PluginViewInfo` round trip/truncation (in
  `src/plugin_abi.rs`).
- The [`src/templates/plugin_shim.rs`](src/templates/plugin_shim.rs) template: its nine `sk_run_sk_*` entry points are
  module-level items, each one exported by the plugin library built from the template.

## 11. Status and history

- **v1** (ABI 1): `sk_run_sk_begin` received a pointer to the host `Sk`; the whole viewer logic lived inside
  `src/bin/cargo-run_sk.rs` (CLI + watcher + swap + UI), validated with the Simulator and Monado.
- **v2** (current, ABI 2): `sk_run_sk_begin` receives a pointer to the `Rc<RefCell<SkInfo>>` of the host session; the
  viewer logic became the reusable `tools::hot_reloading::HotReloading` `IStepper` (the `cargo-run_sk` binary is now
  only the CLI + the `LogWindow`), the fps display and the selector window were reworked, and the workflow was
  extended to Windows/macOS and to the projects generated by `cargo new_sk_rs_project` (template `plugin_shim.rs`,
  forwarded `skc-shared` feature, "Next steps" hint).

## See also

- [`README.md`](README.md) — "Develop with hot-reload, the `cargo run_sk` viewer" (user-facing quick start).
- [`src/plugin_abi.rs`](src/plugin_abi.rs) — the C ABI, documented for both sides.
- [`src/tools/hot_reloading.rs`](src/tools/hot_reloading.rs) — the tool itself, with its module documentation.
- [`src/bin/cargo-run_sk.rs`](src/bin/cargo-run_sk.rs) — the viewer (`--help` output and the main loop).
- [`examples/run_sk_plugin/mod.rs`](examples/run_sk_plugin/mod.rs) and
  [`src/templates/plugin_shim.rs`](src/templates/plugin_shim.rs) — the two plugin-side shims.
