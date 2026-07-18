pub mod demos;

pub const _USAGE: &str = r#"Usage : program [OPTION] 
launch Stereokit tests and demos

    --test              : test mode
    --headless          : no display at all for --test
    --noscreens         : no screenshots
    --screenfolder [DIR]: path where the screenshots will be saved
    --gltf              : path where the gltf files are stored
    --start [TEST NAME] : name of the only test demo to launch
    --log-env           : dump launch environment (env vars, cwd, parent process) to stderr
    --help              : help"#;

pub const USAGE: &str = r#"Usage : program [OPTION] 
    launch Stereokit tests and demos
    
        --test              : test mode
        --headless          : no display at all for --test
        --xr                : force XR mode in testing mode
        --start [TEST NAME] : name of the only test demo to launch
        --log-env           : dump launch environment (env vars, cwd, parent process) to stderr
        --help              : help"#;

/// Logs launch-time information that can explain behavioural differences
/// between launching the program from Steam and launching it directly from
/// Linux. Steam injects environment variables (LD_PRELOAD for the overlay,
/// Vulkan/OpenXR layer paths, locale, XDG dirs...), changes the working
/// directory and argv[0], and runs the process in a different session, any
/// of which can alter the runtime behaviour of StereoKit / OpenXR / Vulkan.
///
/// This is written to **stderr** so it shows up before StereoKit's log is
/// initialized and is unaffected by `log_filter` settings.
#[cfg(not(feature = "no-event-loop"))]
#[cfg(not(target_os = "android"))]
fn log_launch_environment() {
    use std::env;
    use std::process::Command;

    // Environment variables that commonly differ when launched from Steam,
    // the Steam overlay (LD_PRELOAD), Vulkan layers, OpenXR runtime/api
    // layers, SDL/SDL3, XDG base directories, locale, display/wayland, etc.
    const INTERESTING_VARS: &[&str] = &[
        // Steam
        "STEAM_RUNTIME",
        "STEAM_PATH",
        "STEAM_COMPAT_CLIENT_INSTALL_PATH",
        "STEAM_COMPAT_DATA_PATH",
        "STEAM_COMPAT_APP_ID",
        "SteamAppId",
        "SteamGameId",
        "SteamEnv",
        "STEAM_FRAME_FORCE_LISTEN",
        "STEAM_ZENITY",
        // Dynamic loader / overlay
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "LD_AUDIT",
        // Vulkan
        "VK_DRIVER_FILES",
        "VK_ICD_FILENAMES",
        "VK_LAYER_PATH",
        "VK_INSTANCE_LAYERS",
        "VK_DEVICE_LAYERS",
        "VK_EXT_DEBUG_UTILS",
        "VK_LOADER_DEBUG",
        "MESA_VK_WSI_PRESENT_MODE",
        "ENABLE_VK_SAMPLER_REDUCTION",
        "RADV_PERFTEST",
        "RADV_DEBUG",
        // OpenXR
        "XR_RUNTIME_JSON",
        "XR_API_LAYER_PATH",
        "XR_ENABLE_DEBUG_EXTENSION",
        "XR_LOADER_DEBUG",
        // SDL
        "SDL_VIDEODRIVER",
        "SDL_AUDIODRIVER",
        "SDL_JOYSTICK_DEVICE",
        "SDL_GAMECONTROLLERCONFIG",
        "SDL_X11_FORCE_OVERRIDE",
        "SDL_DISABLE_LOCK_KEYS",
        // Display / session
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_SESSION_TYPE",
        "XDG_RUNTIME_DIR",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "DBUS_SESSION_BUS_ADDRESS",
        "PULSE_SERVER",
        "QT_QPA_PLATFORM",
        // Locale
        "LANG",
        "LC_ALL",
        "LC_MESSAGES",
        "LC_CTYPE",
        // Misc that affects graphics/runtime
        "MESA_GL_VERSION_OVERRIDE",
        "MESA_GLSL_VERSION_OVERRIDE",
        "LIBGL_ALWAYS_SOFTWARE",
        "__GL_THREADED_OPTIMIZATIONS",
        "DRI_PRIME",
        "VK_DONT_BLOCK_ON_SUBMITS",
        // Sandbox / pressure-vessel (Proton/Steam Linux Runtime)
        "PRESSURE_VESSEL",
        "PRESSURE_VESSEL_SHELL",
    ];

    let mut out = String::new();
    out.push_str("\n================ LAUNCH ENVIRONMENT ================\n");

    // argv[0] and full argv: Steam sometimes wraps the binary in a launch
    // script, which changes argv[0] and how the process is found.
    let args: Vec<String> = env::args().collect();
    out.push_str(&format!("argv[0]      : {}\n", args.first().map(String::as_str).unwrap_or("(none)")));
    out.push_str(&format!("argv (full)  : {}\n", args.join(" ")));

    // Current executable as resolved by the kernel. Useful to spot wrapper
    // scripts or a different binary than expected.
    if let Ok(exe) = env::current_exe() {
        out.push_str(&format!("current_exe  : {}\n", exe.display()));
    } else {
        out.push_str("current_exe  : <unavailable>\n");
    }

    // Working directory: Steam may launch from its own directory rather than
    // the project root, which breaks relative asset paths.
    if let Ok(cwd) = env::current_dir() {
        out.push_str(&format!("current_dir  : {}\n", cwd.display()));
    } else {
        out.push_str("current_dir  : <unavailable>\n");
    }

    // HOME can differ under Steam Runtime / pressure-vessel (snaphot home).
    out.push_str(&format!("HOME         : {}\n", env::var("HOME").unwrap_or_else(|_| "<unset>".into())));

    // Locale is a classic cause of different formatting/parsing behaviour.
    out.push_str(&format!("LANG         : {}\n", env::var("LANG").unwrap_or_else(|_| "<unset>".into())));
    out.push_str(&format!("LC_ALL       : {}\n", env::var("LC_ALL").unwrap_or_else(|_| "<unset>".into())));

    out.push_str("\n--- selected environment variables ---\n");
    for var in INTERESTING_VARS {
        match env::var(var) {
            Ok(value) => {
                if value.is_empty() {
                    out.push_str(&format!("  {var} = (empty)\n"));
                } else {
                    out.push_str(&format!("  {var} = {value}\n"));
                }
            }
            Err(_) => out.push_str(&format!("  {var} = <unset>\n")),
        }
    }

    // Full environment dump is too noisy for the default log, but a count and
    // the sorted list of *names* is enough to spot missing/extra vars between
    // the two launch contexts. Set LOG_ENV_FULL=1 to dump everything.
    let env_count = env::vars_os().count();
    out.push_str(&format!("\nTotal environment variables: {env_count}\n"));
    let mut names: Vec<String> = env::vars_os().map(|(k, _)| k.to_string_lossy().into_owned()).collect();
    names.sort();
    out.push_str(&format!("Variable names (sorted): {}\n", names.join(", ")));
    if env::var("LOG_ENV_FULL").map(|v| v == "1").unwrap_or(false) {
        out.push_str("\n--- FULL environment dump (LOG_ENV_FULL=1) ---\n");
        let mut all: Vec<(String, String)> = env::vars().collect();
        all.sort();
        for (k, v) in all {
            out.push_str(&format!("  {k}={v}\n"));
        }
    }

    // Parent process: Steam launches through `steam-runtime-launch-client`,
    // `steamapps/common/...`, `pressure-vessel-wrap`, a Proton entry point or
    // a shell wrapper. Knowing the parent chain explains inherited env/limits.
    let pid = std::process::id();
    out.push_str(&format!("\nPID          : {pid}\n"));
    let ppid = std::os::unix::process::parent_id();
    out.push_str(&format!("PPID         : {ppid}\n"));
    {
        // Best-effort: try `ps` to name the parent. This won't be available on
        // all setups but is very informative when it is.
        if let Ok(output) = Command::new("ps").args(["-o", "pid=,ppid=,comm=", "-p"]).arg(ppid.to_string()).output() {
            if output.status.success() {
                let line = String::from_utf8_lossy(&output.stdout);
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    out.push_str(&format!("parent (ps)  : {trimmed}\n"));
                }
            }
        }
        // Full parent chain via ps for deeper inspection.
        if let Ok(output) = Command::new("ps").args(["-o", "pid=,ppid=,comm=", "-g"]).arg(ppid.to_string()).output() {
            if output.status.success() {
                let body = String::from_utf8_lossy(&output.stdout);
                let mut lines: Vec<&str> = body.lines().collect();
                if !lines.is_empty() {
                    out.push_str("parent tree  :\n");
                    for l in &lines {
                        out.push_str(&format!("    {l}\n"));
                    }
                    lines.clear();
                }
            }
        }
    }

    // Process resource limits can differ (Steam Runtime caps some of them),
    // which affects e.g. how many file descriptors / OpenXR layers are usable.
    if let Ok(output) = Command::new("sh").arg("-c").arg("ulimit -a 2>/dev/null").output() {
        if output.status.success() {
            let body = String::from_utf8_lossy(&output.stdout);
            out.push_str("\n--- ulimit -a ---\n");
            out.push_str(&body);
        }
    }

    out.push_str("================ END LAUNCH ENVIRONMENT ================\n");
    eprintln!("{out}");
}

#[allow(dead_code)]
#[cfg(not(feature = "no-event-loop"))]
#[cfg(not(target_os = "android"))]
fn main() {
    use demos::program::launch;
    use std::env;
    use stereokit_rust::sk::{DepthMode, Sk, StandbyMode};
    use stereokit_rust::system::BackendOpenXR;
    use stereokit_rust::{
        sk::{AppMode, OriginMode, SkSettings},
        system::LogLevel,
    };

    let mut headless = false;
    let mut xr = false;
    let mut is_testing = false;
    let mut log_env = false;
    let mut start_test = "".to_string();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match &arg[..] {
            "--headless" => headless = true,
            "--xr" => xr = true,
            "--test" => is_testing = true,
            "--log-env" => log_env = true,

            // "--noscreens" => make_screenshots = false,

            // "--screenfolder" => {
            //     if let Some(arg_config) = args.next() {
            //         if Path::new(&arg_config).is_dir() {
            //             screenshot_root = arg_config;
            //         } else {
            //             panic!("Value specified for --Screenfolder is not a valid Path to a directory.");
            //         }
            //     } else {
            //         panic!("No value specified for parameter --Screenfolder.");
            //     }
            // }
            // "--gltf" => {
            //     if let Some(arg_config) = args.next() {
            //         if Path::new(&arg_config).is_dir() {
            //             gltf_folders = arg_config;
            //         } else {
            //             panic!("Value specified for --gltf is not a valid Path to a directory.");
            //         }
            //     } else {
            //         panic!("No value specified for parameter --gltf.");
            //     }
            // }
            "--start" => {
                if let Some(arg_config) = args.next() {
                    if !arg_config.starts_with('-') {
                        start_test = arg_config;
                    } else {
                        panic!("Value specified for --start must be the name of a test.");
                    }
                } else {
                    panic!("No value specified for parameter --start.");
                }
            }
            "--help" => println!("{USAGE}"),
            _ => {
                if arg.starts_with('-') {
                    println!("Unkown argument {arg}");
                } else {
                    println!("Unkown positional argument {arg}");
                }
                println!("{USAGE}");
            }
        }
    }

    if log_env {
        log_launch_environment();
    }

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Floor)
        .render_multisample(4) // aka the default aka 0
        //.render_scaling(1.5) create distortion on SteamVR for Quest
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic)
        .no_flatscreen_fallback(true);

    if is_testing {
        if headless {
            settings.mode(AppMode::Offscreen);
        } else if xr {
            settings.mode(AppMode::XR);
        } else {
            settings.mode(AppMode::Simulator);
        }
    }
    settings.standby_mode(StandbyMode::Slow);

    //sterokit_rust::tools::load_all_extensions();
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_FB_render_model");
    // Required by the Layers1 demo for cylinder composition layers.
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");

    let sk = settings.init().unwrap();
    launch(sk, is_testing, start_test);
    Sk::shutdown();
}

/// Fake main for android
#[allow(dead_code)]
#[cfg(target_os = "android")]
fn main() {}

#[allow(dead_code)]
#[cfg(feature = "no-event-loop")]
fn main() {
    use std::sync::OnceLock;
    use stereokit_rust::{
        maths::{Pose, Quat, Vec3},
        sk::{OriginMode, Sk, SkSettings},
        system::LogLevel,
        ui::Ui,
    };

    let sk = SkSettings::default()
        .app_name("stereokit-rust (manual)")
        .origin(OriginMode::Floor)
        .log_filter(LogLevel::Diagnostic)
        .init()
        .unwrap();

    static APP_ONCE: OnceLock<()> = OnceLock::new();
    APP_ONCE.get_or_init(|| {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
        );
    });
    let mut window_pose = Pose::new(Vec3::new(0.0, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0)));
    while let Some(_token) = sk.step() {
        Ui::window("test window").pose(&mut window_pose).begin();
        if Ui::button("quit lel").press() {
            break;
        }
        Ui::window_end();
    }
    Sk::shutdown();
}
