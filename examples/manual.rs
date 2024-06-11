#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::{
        maths::{Pose, Quat, Vec3},
        sk::{OriginMode, SkSettings},
        system::LogLevel,
        ui::Ui,
    };

    let sk = SkSettings::default()
        .app_name("stereokit-rust (manual)")
        .assets_folder("assets")
        .origin(OriginMode::Floor)
        .log_filter(LogLevel::Diagnostic)
        .init()
        .unwrap();

    let mut window_pose = Pose::new(Vec3::new(0.0, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0)));
    while let Some(_token) = sk.step() {
        Ui::window_begin("test window", &mut window_pose, None, None, None);
        if Ui::button("quit lel", None) {
            break;
        }
        Ui::window_end();
    }
}

/// Fake main for android
#[allow(dead_code)]
#[cfg(target_os = "android")]
fn main() {}
