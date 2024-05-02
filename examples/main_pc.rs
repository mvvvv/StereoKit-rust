pub mod demos;

use std::env;

use demos::program::launch;
use stereokit_rust::{
    sk::{AppMode, OriginMode, SkSettings},
    system::LogLevel,
};

pub const _USAGE: &str = r#"Usage : program [OPTION] 
launch Stereokit tests and demos

    --test              : test mode
    --headless          : no display at all for --test
    --noscreens         : no screenshots
    --screenfolder [DIR]: path where the screenshots will be saved
    --gltf              : path where the gltf files are stored
    --start [TEST NAME] : name of the only test demo to launch
    --help              : help"#;

pub const USAGE: &str = r#"Usage : program [OPTION] 
    launch Stereokit tests and demos
    
        --test              : test mode
        --headless          : no display at all for --test
        --start [TEST NAME] : name of the only test demo to launch
        --help              : help"#;

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::system::BackendOpenXR;

    let mut headless = false;
    let mut is_testing = false;
    let mut start_test = "".to_string();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match &arg[..] {
            "--headless" => headless = true,
            "--test" => is_testing = true,

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
            "--help" => println!("{}", USAGE),
            _ => {
                if arg.starts_with('-') {
                    println!("Unkown argument {}", arg);
                } else {
                    println!("Unkown positional argument {}", arg);
                }
                println!("{}", USAGE);
            }
        }
    }
    let mut settings = SkSettings::default();
    settings
        .app_name("stereokit-rust")
        .assets_folder("assets")
        .origin(OriginMode::Floor)
        .log_filter(LogLevel::Diagnostic)
        .no_flatscreen_fallback(true);

    if is_testing {
        if headless {
            settings.mode(AppMode::Offscreen);
        } else {
            settings.mode(AppMode::Simulator);
        }
        settings.disable_unfocused_sleep(true);
    }

    BackendOpenXR::request_ext("XR_FB_passthrough");

    let (sk, event_loop) = settings.init().unwrap();
    launch(sk, event_loop, is_testing, start_test);
}
