//! `main_hot_reloading` — a real-time viewer with in-process hot-reload for StereoKit-rust projects (Linux, Windows &
//! macOS).
//!
//! The host keeps **one** StereoKit session alive (Simulator by default, OpenXR with `--xr`, offscreen with
//! `--offscreen`), and the project under development is loaded as a plugin library.
//!
//! # Typical workflow (this repository)
//! ```bash
//! # terminal 1 - the viewer (Simulator by default)
//! cargo run --bin main_hot_reloading --features skc-shared
//! ```
//! Fully automatic: by default the viewer also watches `src/bin/` and `examples/` and runs the build command itself at
//! startup and then on each change (`--no-build` to disable, `--build-cmd` to customize).
//!
//! With the Monado simulator runtime:
//! ```bash
//! XR_RUNTIME_JSON=/usr/share/openxr/1/openxr_monado.json main_hot_reloading --xr
//! ```

#[cfg(feature = "no-event-loop")]
fn main() {
    eprintln!("main_hot_reloading: only supported with event-loop");
}

#[cfg(not(feature = "no-event-loop"))]
fn main() {
    imp::main();
}

#[cfg(all(any(target_os = "linux", target_os = "windows", target_os = "macos"), not(feature = "no-event-loop")))]
mod imp {
    use std::{path::PathBuf, sync::Mutex};

    use stereokit_rust::{
        framework::{SkClosures, StepperAction},
        maths::{Pose, Quat, Vec3},
        sk::{AppMode, Sk, SkSettings},
        system::{Log, LogItem, LogLevel},
        tools::{
            hot_reloading::{HotReloading, default_build_cmd, default_lib_path, plugin_file},
            log_window::{LogWindow, basic_log_fmt},
        },
    };

    /// Somewhere to copy the log, read by the viewer's [`LogWindow`].
    static LOG_LOG: Mutex<Vec<LogItem>> = Mutex::new(vec![]);

    pub const USAGE: &str = r#"Usage : main_hot_reloading [OPTION]
    Real-time viewer for the project under development: keeps a single
    StereoKit session alive (Simulator or OpenXR) and hot-reloads the
    plugin library each time it is rebuilt.

        --simulator            : start in Simulator mode (default)
        --xr                   : start in OpenXR mode (no flatscreen fallback)
        --offscreen            : start without any display (CI / screenshots)
        --lib <path>           : plugin library to load
                                 (the COMPILED plugin: <name>.dll on Windows,
                                 lib<name>.so on Linux, lib<name>.dylib on macOS;
                                 default: detected from the sources present:
                                 src/bin/main_*.rs -> target/debug/<plugin>,
                                 examples/main*.rs -> target/debug/examples/<plugin>,
                                 else the first plugin file under target/debug/examples)
        --start <view name>    : view to select at start (name, or substring)
        --watch <secs>         : plugin library watch period, 0 disables (default 0.5)
        --build-cmd <command>  : build command run at startup and on source change.
                                 Default derived from the project layout (with the
                                 example name found in examples/ for this repo):
                                 "cargo build --example <name> --features skc-shared"
                                 or "cargo build --lib --features skc-shared"
        --no-build             : don't watch sources / don't auto-build
        --test [N steps]       : run N steps (default 1000), screenshot the
                                 active view then exit
        --list                 : print the views of the plugin and exit
        --fullscreen           : ask the desktop window to start fullscreen
        -h | --help            : display this help

    Examples:
        main_hot_reloading
        main_hot_reloading --xr --start Tex1
        main_hot_reloading --offscreen --test 120 --start Ui1"#;

    // ------------------------------------------------------------------
    // main
    // ------------------------------------------------------------------

    /// The command line of the viewer: it configures the [`HotReloading`] tool of the project, initializes the
    /// session with the settings of the project (read from the plugin), and runs the main loop.
    pub fn main() {
        //---- First the command line (style of the other cargo-* binaries)
        let mut fullscreen = false;
        let mut xr = false;
        let mut offscreen = false;
        let mut list = false;
        let mut no_build = false;
        let mut start_view = String::new();
        let mut lib_path = default_lib_path();
        let mut watch_secs = 0.5f32;
        let mut build_cmd: Option<String> = None;
        let mut test_steps: u32 = 0;

        let argv: Vec<String> = std::env::args().skip(1).collect();
        let mut index = 0;
        while let Some(arg) = argv.get(index).cloned() {
            index += 1;
            match &arg[..] {
                // the binary name itself may be passed as first argument
                "main_hot_reloading" => {}
                "--simulator" => {}
                "--xr" => xr = true,
                "--offscreen" => offscreen = true,
                "--fullscreen" => fullscreen = true,
                "--list" => list = true,
                "--no-build" => no_build = true,
                "--lib" => match argv.get(index) {
                    Some(value) if !value.starts_with('-') => {
                        lib_path = PathBuf::from(value);
                        index += 1;
                    }
                    _ => panic!("No value specified for parameter --lib."),
                },
                "--start" => match argv.get(index) {
                    Some(value) if !value.starts_with('-') => {
                        start_view = value.clone();
                        index += 1;
                    }
                    _ => panic!("No value specified for parameter --start."),
                },
                "--watch" => match argv.get(index) {
                    Some(value) if !value.starts_with('-') => {
                        watch_secs = value.parse().unwrap_or_else(|_| panic!("--watch expects a number of seconds"));
                        index += 1;
                    }
                    _ => panic!("No value specified for parameter --watch."),
                },
                "--build-cmd" => match argv.get(index) {
                    Some(value) if !value.starts_with('-') => {
                        build_cmd = Some(value.clone());
                        index += 1;
                    }
                    _ => panic!("No value specified for parameter --build-cmd."),
                },
                "--test" => {
                    test_steps = 1000;
                    if let Some(value) = argv.get(index)
                        && !value.starts_with('-')
                    {
                        test_steps = value.parse().unwrap_or_else(|_| panic!("--test expects a number of steps"));
                        index += 1;
                    }
                }
                "-h" | "--help" => println!("{USAGE}"),
                _ => {
                    if arg.starts_with('-') {
                        println!("Unknown argument {arg}");
                    } else {
                        println!("Unknown positional argument {arg}");
                    }
                    println!("{USAGE}");
                    return;
                }
            }
        }

        //---- Guard: a frequent mistake is to pass the plugin SOURCE to --lib instead of the
        // compiled plugin library: LoadLibrary/dlopen would fail on the .rs file in a confusing way.
        if lib_path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rs")) {
            eprintln!(
                "main_hot_reloading: --lib expects the COMPILED plugin library (e.g. target/debug/examples/{}, \
                 not the Rust source file {}). Omit --lib to let the viewer detect and build it itself.",
                plugin_file("main"),
                lib_path.display()
            );
            println!("{USAGE}");
            return;
        }

        //---- --list: read the views of the plugin and exit, without any StereoKit session
        if list {
            return match HotReloading::list_views(&lib_path) {
                Ok(views) => {
                    println!("main_hot_reloading: {} views in {}", views.len(), lib_path.display());
                    for (index, (name, has_screenshot)) in views.iter().enumerate() {
                        println!("  [{index:2}] {name} {}", if *has_screenshot { "[screenshot]" } else { "" });
                    }
                }
                Err(err) => {
                    eprintln!("main_hot_reloading: {err}");
                    std::process::exit(1);
                }
            };
        }

        //---- The hot reloading tool of the project: it watches the sources, runs the build command, loads (and
        // reloads) the plugin, and offers its views in a selector window.
        let mut hot_reloading = HotReloading::new(&lib_path)
            .build_cmd(build_cmd.unwrap_or_else(|| default_build_cmd(&lib_path)))
            .window_pose(Pose::new(Vec3::new(-0.7, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))))
            .watch(watch_secs)
            .test_steps(test_steps);
        if !start_view.is_empty() {
            hot_reloading = hot_reloading.start_view(start_view);
        }
        if no_build {
            hot_reloading = hot_reloading.no_build();
        }

        //---- The session settings are the ones of the project: the tool reads them from the plugin BEFORE StereoKit
        // is initialized.
        let mut settings = SkSettings::default();
        if let Err(err) = hot_reloading.apply_plugin_settings(&mut settings) {
            Log::warn(format!("main_hot_reloading: {err}"));
        }

        //---- The launch-dependent settings stay in the host: they are applied on top of the project ones.
        settings.app_name("main_hot_reloading").fullscreen(fullscreen);
        if xr {
            settings.mode(AppMode::XR).no_flatscreen_fallback(true);
        } else if offscreen {
            settings.mode(AppMode::Offscreen);
        } else {
            settings.mode(AppMode::Simulator);
        }
        //---- The log window of the viewer: subscribed before the session starts, so the StereoKit initialization
        // logs are captured too.
        let fn_mut = |level: LogLevel, log_text: &str| {
            let items = LOG_LOG.lock().expect("Failed to lock log mutex");
            basic_log_fmt(level, log_text, items);
        };
        Log::subscribe(fn_mut);

        let mut sk = settings.init().expect("main_hot_reloading: cannot initialize StereoKit");

        //---- The viewer is now only this tool: it is stepped after the app callback, like the explicit call it
        // replaces.
        sk.send_event(StepperAction::add("hot_reloading", hot_reloading));

        //---- The log window, on the left of the selector window: the developer sees the logs of the session
        // without leaving the viewer (build tails, view selection, plugin messages...).
        let mut log_window = LogWindow::new(&LOG_LOG);
        log_window.window_pose = Pose::new(Vec3::new(-0.7, 2.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));
        sk.send_event(StepperAction::add("LogWindow", log_window));

        //---- Main loop
        SkClosures::new(sk, |_sk, _token| {}).run();

        Sk::shutdown();
    }
}
