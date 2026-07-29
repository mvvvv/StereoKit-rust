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

/// Fake main for android
#[allow(dead_code)]
#[cfg(target_os = "android")]
fn main() {}

/// Fake main for event-loop  (rust-analyzer problem as event-loop is the defaut feature )
#[allow(dead_code)]
#[cfg(not(feature = "no-event-loop"))]
fn main() {
    panic!("This example works with feature `no_event_loop`!");
}
