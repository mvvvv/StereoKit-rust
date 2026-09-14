#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::sk::Sk;
    use vr_app::{launch, sk_settings};
    // Initialize StereoKit with the settings grouped in the sk_settings() function of the project (lib.rs)
    let settings = sk_settings();

    // Main loop
    launch(settings, false);

    Sk::shutdown();
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
//fake main fn for android as entry is lib.rs/android_main(...)
fn main() {}
