#[cfg(not(target_os = "android"))]
use std::env;

#[cfg(not(target_os = "android"))]
use stereokit_rust::{
    sk::{AppMode, OriginMode, SkSettings},
    system::LogLevel,
};

pub const USAGE: &str = r#"Usage : program [OPTION] 
    launch Stereokit tests and demos
    
        --test              : test mode
        --headless          : no display at all for --test
        --help              : help"#;

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
/// The main function when launched on PC. Set --test to use the simulator
fn main() {
    use stereokit_rust::sk::{DepthMode, Sk, StandbyMode};
    use vr_app::launch;

    let mut headless = false;
    let mut is_testing = false;
    let args = env::args().skip(1);
    for arg in args {
        match &arg[..] {
            "--headless" => headless = true,
            "--test" => is_testing = true,
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
    let mut settings = SkSettings::default();
    settings
        .app_name("Template App")
        .origin(OriginMode::Floor)
        .render_scaling(2.0)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    if is_testing {
        if headless {
            settings.mode(AppMode::Offscreen);
        } else {
            settings.mode(AppMode::Simulator);
        }
    }
    settings.standby_mode(StandbyMode::None);

    let sk = settings.init().unwrap();
    launch(sk, is_testing);
    Sk::shutdown();
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
//fake main fn for android because it will use lib.rs/android_main(...)
fn main() {}
