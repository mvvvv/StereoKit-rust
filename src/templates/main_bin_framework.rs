#[cfg(not(target_os = "android"))]
use std::env;

#[cfg(not(target_os = "android"))]
use stereokit_rust::sk::AppMode;

pub const USAGE: &str = r#"Usage : program [OPTION] 
    launch Stereokit tests and demos
    
        --test              : test mode
        --headless          : no display at all for --test
        --help              : help"#;

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
/// The main function when launched on PC. Set --test to use the simulator
fn main() {
    use stereokit_rust::sk::{Sk, StandbyMode};
    use vr_app::{launch, sk_settings};

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
    // All the SkSettings AND the BackendOpenXR / BackendVulkan parameterizations are grouped in sk_settings()
    let mut settings = sk_settings();

    if is_testing {
        if headless {
            settings.mode(AppMode::Offscreen);
        } else {
            settings.mode(AppMode::Simulator);
        }
    }
    settings.standby_mode(StandbyMode::None);

    // main loop
    launch(settings, is_testing);

    Sk::shutdown();
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
//fake main fn for android because it will use lib.rs/android_main(...)
fn main() {}
