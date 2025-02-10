use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

//use proc_macro::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use proc_macro::{TokenStream, TokenTree};

use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
///   - **sk_info**: Option<Rc<RefCell<SkInfo>>>,
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
///
/// see the example CStepper in the examples/demos/c_stepper.rs
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
    let stringified = format!("&{:?}", vec_path);
    TokenStream::from_str(&stringified).unwrap()

    // let body = [TokenTree::Literal(Literal::string("/assets"))].into_iter().collect();
    // [
    //     TokenTree::Punct(Punct::new('&', Spacing::Alone)),
    //     TokenTree::Group(Group::new(Delimiter::Bracket, body)),
    // ]
    // .into_iter()
    // .collect()
}

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

#[proc_macro]
/// Initialize sk (and eventually event_loop) for a test.
///
/// If you intend to run a main loop, with test_screenshot!(...) or test_steps!(...) here some variables you may use:
/// * number_of_steps - Default is 1, you can change this value before the main loop
/// * token - the MainThreadToken you need to draw in the main_loop
/// * iter - The step number in the main_loop. [0..number_of_steps]
///
/// If you intend to take a screenshot with test_screenshot!(...) there is also those variables to change before the
/// main loop:
/// * width_scr - width of the screenshot (default is 200)
/// * height_scr - height of the screenshot (default is 200)
/// * fov_scr - fov of the screenshot (default is 99.0)
/// * from_scr - Position of the camera (default is Vec3::Z)
/// * at_scr - Point looked at by the camera (default is Vec3::ZERO)
pub fn test_init_sk(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        use stereokit_rust::{*, prelude::*, test_screenshot, test_steps};

        #[cfg(feature = "no-event-loop")]
        let mut sk = sk::SkSettings::default().mode(sk::AppMode::Offscreen).app_name("cargo test").init().unwrap();
        #[cfg(feature = "event-loop")]
        let (mut sk, mut event_loop) = sk::SkSettings::default().mode(sk::AppMode::Offscreen).app_name("cargo test").init_with_event_loop().unwrap();

        let mut filename_scr = "screenshots/default_screenshoot.png";
        let mut number_of_steps = 1;
        let (mut width_scr, mut height_scr, mut fov_scr, mut from_scr, mut at_scr)  = (200, 200, 99.0, maths::Vec3::Z, maths::Vec3::ZERO);
    };

    TokenStream::from(expanded)
}

#[proc_macro]
/// Run a main_loop then take a screenshot when the number_of_steps is reached
/// see [`crate::test_init_sk`] for the details
pub fn test_screenshot(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let expanded = quote! {
        let mut iter = 0;
        #[cfg(feature = "no-event-loop")]
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
        #[cfg(feature = "event-loop")]
        {
            event_loop::SkClosures::new(sk, |sk, token| {
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
/// Run a main_loop until the number_of_steps is reached
/// see [`crate::test_init_sk`] for the details
pub fn test_steps(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let expanded = quote! {
        let mut iter = 0;
        #[cfg(feature = "no-event-loop")]
        {
            while let Some(token) = sk.step() {
                if iter > number_of_steps {break}

                #input

                iter+=1;
            }
        }
        #[cfg(feature = "event-loop")]
        {
            event_loop::SkClosures::new(sk, |sk, token| {
                if iter > number_of_steps {sk.quit(None)}

                #input

                iter+=1;
            }).run(event_loop);
        }
    };

    TokenStream::from(expanded)
}
