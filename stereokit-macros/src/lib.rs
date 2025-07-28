use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

//use proc_macro::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use proc_macro::{TokenStream, TokenTree};

use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

// Check if the struct has a field named field_name
fn has_field(field_name: &str, input: &DeriveInput) -> bool {
    match input.data {
        Data::Struct(ref data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => fields_named.named.iter().any(|f| f.ident.as_ref().unwrap() == field_name),
            _ => false,
        },
        _ => false,
    }
}

/// Derive the IStepper trait for a struct which must implement:
/// * Fields:     
///   - **id**: StepperId,
///   - **sk_info**: `Option<Rc<RefCell<SkInfo>>>`,
///   - *Optional* when the stepper should initialize on more than one step : **initialize_completed**: bool
///   - *Optional* when you want to implement an active/inactive flag: **enabled**: bool
///   - *Optional* when the stepper should shutdown some stuffs : **shutdown_completed**: bool
/// * Functions:
///   - IStepper::initialize calls **fn start(&mut self) -> bool** where you can abort the initialization by returning false:
///   - *Optional* if field **initialize_completed** is present IStepper::initialize_done calls
///     **fn start_completed(&mut self) -> bool** where you can tell the initialization is done:
///   - IStepper::step calls  **fn check_event(&mut self, _key: &str, _value: &str)** where you can check the event report:
///   - IStepper::step calls **fn draw(&mut self, token: &MainThreadToken)** after check_event where you can draw your UI:
///   - *Optional* if field **shutdown_completed** is present IStepper::shutdown and IStepper::shutdown_done call
///     **fn close(&mut self, triggering:bool) -> bool**
///     where you can close your resources.
///     
/// ### Examples
/// ```ignore
/// use stereokit_rust::{prelude::*, material::Material, maths::{Matrix, Quat, Vec3},
///                      mesh::Mesh, util::{named_colors, Time}};
/// #[derive(IStepper)]
/// pub struct MyStepper {
///     id: StepperId,
///     sk_info: Option<Rc<RefCell<SkInfo>>>,
///
///     transform: Matrix,
///     round_cube: Mesh,
///     material: Material,
/// }
/// impl Default for MyStepper {
///     fn default() -> Self {
///         Self {
///             id: "MyStepper".to_string(),
///             sk_info: None,
///
///             transform: Matrix::IDENTITY,
///             round_cube: Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.2, Some(16)),
///             material: Material::pbr().copy(),
///         }
///     }
/// }
/// impl MyStepper {
///     fn start(&mut self) -> bool {
///         self.transform = Matrix::r([0.0, 10.0 * Time::get_stepf(), 0.0]);
///         self.material.color_tint(named_colors::BLUE);
///         true
///     }
///     fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}
///     fn draw(&mut self, token: &MainThreadToken) {
///         self.round_cube.draw(token, &self.material, self.transform, None, None);
///     }
/// }
///  ```
///
/// see also the example CStepper in the examples/demos/c_stepper.rs
#[proc_macro_derive(IStepper)]
pub fn derive_istepper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;

    let init_completed = if has_field("initialize_completed", &input) {
        quote! {
            fn initialize_done(&mut self) -> bool {
                self.start_completed()
            }
        }
    } else {
        quote! {}
    };

    let enabled_fn = if has_field("enabled", &input) {
        quote! {
            fn enabled(&self) -> bool {
                self.enabled
            }
        }
    } else {
        quote! {}
    };

    let close_fn = if has_field("shutdown_completed", &input) {
        quote! {

            fn shutdown(&mut self) {
                self.close(true);
            }

            fn shutdown_done(&mut self) -> bool {
                self.close(false)
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #generics IStepper for #name #generics {

            #init_completed

            #enabled_fn

            fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
                self.id = id;
                self.sk_info = Some(sk_info);

                self.start()
            }

            fn step(&mut self, token: &MainThreadToken) {

                for e in token.get_event_report().iter() {
                    if let StepperAction::Event(id, key, value) = e {
                        self.check_event(id, key, value);
                    }
                }

                if !self.enabled() {
                    return;
                };

                self.draw(token)
            }

            #close_fn

        }
    };

    TokenStream::from(expanded)
}

/// Embed the tree of the assets sub-directories in your crate.
/// useful if you want to browse some assets
/// * `body` - the path of the assets sub-directory.
///
/// ### Example
/// ``` ignore
/// use stereokit_macros::include_asset_tree;
/// const ASSET_DIR: &[&str] = include_asset_tree!("assets");
///
/// assert_eq!(ASSET_DIR[0], "assets");
/// assert_eq!(ASSET_DIR[1], "assets/textures");
/// ```
#[proc_macro]
pub fn include_asset_tree(body: TokenStream) -> TokenStream {
    let mut vec_path = vec![];
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").ok().unwrap();
    let path_cargo = Path::new(&cargo_dir);
    if let Some(TokenTree::Literal(dir)) = body.into_iter().next() {
        let mut sub_dir = dir.to_string();
        sub_dir.remove(0);
        sub_dir.pop();
        let path_assets = path_cargo.join(&sub_dir);
        if path_assets.is_dir() {
            let sub_path = Path::new(&sub_dir).to_owned();
            vec_path.append(&mut get_sub_dirs(path_assets, &sub_path))
        } else {
            vec_path.push("!!No asset dir tree!!".to_string());
        }
    }
    let stringified = format!("&{vec_path:?}");
    TokenStream::from_str(&stringified).unwrap()

    // let body = [TokenTree::Literal(Literal::string("/assets"))].into_iter().collect();
    // [
    //     TokenTree::Punct(Punct::new('&', Spacing::Alone)),
    //     TokenTree::Group(Group::new(Delimiter::Bracket, body)),
    // ]
    // .into_iter()
    // .collect()
}

/// Dive into a sub directory and get all sub directories.
fn get_sub_dirs(path_assets: PathBuf, sub_path: &Path) -> Vec<String> {
    let mut vec_path = vec![];
    if path_assets.exists() && path_assets.is_dir() {
        vec_path.push(sub_path.to_string_lossy().to_string().replace("\\", "/"));
        if let Ok(read_dir) = read_dir(path_assets) {
            for file in read_dir.flatten() {
                let path_sub_assets = file.path();

                if path_sub_assets.is_dir() {
                    let sub_sub_path = &sub_path.join(file.file_name());
                    vec_path.append(&mut get_sub_dirs(path_sub_assets, sub_sub_path));
                }
            }
        }
    } else {
        vec_path.push("!!No asset sub dir tree!!".to_string())
    }
    vec_path
}

/// StereoKit-rust renames this macro to `test_init_sk!`.
/// Initialize sk (and eventually event_loop) for a test.
///
/// If you intend to run a main loop, with `test_screenshot!(...)` or `test_steps!(...)` here some variables you may use:
/// * `number_of_steps` - Default is 3, you can change this value before the main loop.
/// * `token` - the MainThreadToken you need to draw in the main_loop.
/// * `iter` - The step number in the main_loop. [0..number_of_steps + 2].
///
/// If you intend to take a screenshot with `test_screenshot!(...)` there is also those variables to change before the
/// main loop:
/// * `width_scr` - width of the screenshot (default is 200)
/// * `height_scr` - height of the screenshot (default is 200)
/// * `fov_scr` - fov of the screenshot (default is 99.0)
/// * `from_scr` - Position of the camera (default is Vec3::Z)
/// * `at_scr` - Point looked at by the camera (default is Vec3::ZERO)
///
/// most of the examples of this doc use this macro.
#[proc_macro]
pub fn test_init_sk_event_loop(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        use stereokit_rust::{*, prelude::*, test_screenshot, test_steps, xr_mode_stop_here, offscreen_mode_stop_here};
        let mut sk_settings = sk::SkSettings::default();
        #[cfg(feature = "test-xr-mode")]
        sk_settings.mode(sk::AppMode::XR).app_name("cargo test");
        #[cfg(not(feature = "test-xr-mode"))]
        sk_settings.mode(sk::AppMode::Offscreen).app_name("cargo test");
        let (mut sk, mut event_loop) = sk_settings.init_with_event_loop().unwrap();

        let mut filename_scr = "screenshots/default_screenshoot.png";
        let mut number_of_steps = 3;
        let (mut width_scr, mut height_scr, mut fov_scr, mut from_scr, mut at_scr)  = (200, 200, 99.0, maths::Vec3::Z, maths::Vec3::ZERO);
        system::Assets::block_for_priority(i32::MAX);
    };

    TokenStream::from(expanded)
}

/// StereoKit-rust renames this macro to `test_init_sk!`.
/// Initialize sk (and eventually event_loop) for a test.
///
/// If you intend to run a main loop, with `test_screenshot!(...)` or `test_steps!(...)` here some variables you may use:
/// * `number_of_steps` - Default is 3, you can change this value before the main loop.
/// * `token` - the MainThreadToken you need to draw in the main_loop.
/// * `iter` - The step number in the main_loop. [0..number_of_steps + 2].
///
/// If you intend to take a screenshot with `test_screenshot!(...)` there is also those variables to change before the
/// main loop:
/// * `width_scr` - width of the screenshot (default is 200)
/// * `height_scr` - height of the screenshot (default is 200)
/// * `fov_scr` - fov of the screenshot (default is 99.0)
/// * `from_scr` - Position of the camera (default is Vec3::Z)
/// * `at_scr` - Point looked at by the camera (default is Vec3::ZERO)
///
/// most of the examples of this doc use this macro.
#[proc_macro]
pub fn test_init_sk_no_event_loop(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        use stereokit_rust::{*, prelude::*, test_screenshot, test_steps, xr_mode_stop_here, offscreen_mode_stop_here};
        let mut sk_settings = sk::SkSettings::default();
        #[cfg(feature = "test-xr-mode")]
        sk_settings.mode(sk::AppMode::XR).app_name("cargo test");
        #[cfg(not(feature = "test-xr-mode"))]
        sk_settings.mode(sk::AppMode::Offscreen).app_name("cargo test");
        let mut sk = sk_settings.init().unwrap();

        let mut filename_scr = "screenshots/default_screenshoot.png";
        let mut number_of_steps = 3;
        let (mut width_scr, mut height_scr, mut fov_scr, mut from_scr, mut at_scr)  = (200, 200, 99.0, maths::Vec3::Z, maths::Vec3::ZERO);
        system::Assets::block_for_priority(i32::MAX);
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// Stop execution (Return) if the `test-xr-mode` feature is active. This macro is useful to exit tests early when running
/// in XR mode. Use this when you can't predict what every OpenXR implementation will do.
///
/// ### Example
/// ```ignore
/// #[test]
/// fn my_test() {
///     test_init_sk!();
///     
///     // Code that works in both Offscreen and XR modes
///     // ...
///     
///     xr_mode_stop_here!(); // Exit here if in XR mode
///     
///     // Code that should only run in Offscreen mode
///     // (e.g., pixel-perfect assertions, screenshots)
///     // ...
/// }
/// ```
pub fn xr_mode_stop_here(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        #[cfg(feature = "test-xr-mode")]
        return;
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// Stop execution (Return) if the `test-xr-mode` feature is inactive. This macro is useful to exit tests early when running
/// in offscreen mode. Use this when the following code is only valid in XR mode.
///
/// ### Example
/// ```ignore
/// #[test]
/// fn my_test() {
///     test_init_sk!();
///     
///     // Code that works in both Offscreen and XR modes
///     // ...
///     
///     offscreen_mode_stop_here!(); // Exit here if in offscreen mode
///     
///     // Code that should only run in XR mode
///     // (e.g., pixel-perfect assertions, screenshots)
///     // ...
/// }
/// ```
pub fn offscreen_mode_stop_here(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        #[cfg(not(feature = "test-xr-mode"))]
        return;
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// StereoKit-rust renames this macro to `test_screenshot!`.
/// Run a main_loop then take a screenshot when `iter` equal the `number_of_steps`.
/// see [`crate::test_init_sk!`](crate::test_init_sk_event_loop!) for the details.
pub fn test_screenshot_event_loop(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let expanded = quote! {
        stereokit_rust::system::Assets::block_for_priority(i32::MAX);
        let mut iter = 0;
        {
            framework::SkClosures::new(sk, |sk, token| {
                if iter > number_of_steps {sk.quit(None)}

                #input

                iter+=1;
                if iter == number_of_steps {
                    // render screenshot
                    system::Renderer::screenshot(token, filename_scr, 90, maths::Pose::look_at(from_scr, at_scr), width_scr, height_scr, Some(fov_scr) );
                }
            }).run(event_loop);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// StereoKit-rust renames this macro to `test_screenshot!`.
/// Run a main_loop then take a screenshot when `iter` equal the `number_of_steps`.
/// see [`crate::test_init_sk!`](crate::test_init_sk_no_event_loop!) for the details.
pub fn test_screenshot_no_event_loop(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let expanded = quote! {
        stereokit_rust::system::Assets::block_for_priority(i32::MAX);
        let mut iter = 0;
        {
            while let Some(token) = sk.step() {
                if iter > number_of_steps {break}

                #input

                iter+=1;
                if iter == number_of_steps {
                    // render screenshot
                    system::Renderer::screenshot(token, filename_scr, 90, maths::Pose::look_at(from_scr, at_scr), width_scr, height_scr, Some(fov_scr) );
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// StereoKit-rust renames this macro to `test_steps!`.
/// Run a main_loop until `iter` equal the `number_of_steps`.
/// see [`crate::test_init_sk!`](crate::test_init_sk_event_loop!) for the details.
pub fn test_steps_event_loop(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let expanded = quote! {
        let mut iter = 0;
        {
            framework::SkClosures::new(sk, |sk, token| {
                if iter > number_of_steps {sk.quit(None)}

                #input

                iter+=1;
            }).run(event_loop);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// StereoKit-rust renames this macro to `test_steps!`.
/// Run a main_loop until `iter` equal the `number_of_steps`.
/// see [`crate::test_init_sk!`](crate::test_init_sk_no_event_loop!) for the details.
pub fn test_steps_no_event_loop(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let expanded = quote! {
        let mut iter = 0;
        {
            while let Some(token) = sk.step() {
                if iter > number_of_steps {break}

                #input

                iter+=1;
            }
        }
    };

    TokenStream::from(expanded)
}
