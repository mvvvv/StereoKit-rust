#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
#[cfg(feature = "event-loop")]
fn main() {
    use stereokit_rust::{
        maths::{Pose, Quat, Vec3},
        sk::{OriginMode, Sk, SkSettings},
        sound::Sound,
        system::LogLevel,
        ui::Ui,
    };

    {
        let (sk, _event_loop) = SkSettings::default()
            .app_name("stereokit-rust (manual)")
            .origin(OriginMode::Floor)
            .log_filter(LogLevel::Diagnostic)
            .init_with_event_loop()
            .unwrap();

        let position = Vec3::new(-0.5, 0.0, 0.5);
        let mut plane_sound = Sound::from_file("sounds/no.wav").expect("no.wav should be in the sounds folder");
        assert_eq!(plane_sound.get_id(), "sounds/no.wav");
        plane_sound.id("sound_plane").decibels(90.0);
        plane_sound.play(position, Some(1.0));

        let mut window_pose = Pose::new(Vec3::new(0.0, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0)));
        while let Some(_token) = sk.step() {
            Ui::window_begin("test window", &mut window_pose, None, None, None);
            if Ui::button("quit lel", None) {
                break;
            }
            Ui::window_end();
        }
    }
    Sk::shutdown();
}

/// Fake main for android
#[allow(dead_code)]
#[cfg(target_os = "android")]
fn main() {}

/// Fake main for event-loop  (rust-analyzer problem as event-loop is the defaut feature )
#[allow(dead_code)]
#[cfg(feature = "no-event-loop")]
fn main() {
    panic!("This example works with feature `no_event_loop`!");
}
