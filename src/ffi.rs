use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{library_filename, Library};

#[macro_export]
macro_rules! import_from_c {
    ($fn_name:ident, $symbol_name:expr, $ret_ty:ty, ($($arg_name:ident : $arg_ty:ty),*)) => {
        paste::paste! {
            static [<fn_lock_ $fn_name>]: std::sync::OnceLock<
                libloading::Symbol<unsafe extern "C" fn($($arg_ty),*) -> $ret_ty>
            > = std::sync::OnceLock::new();

            pub unsafe fn $fn_name($($arg_name: $arg_ty),*) -> $ret_ty {
                unsafe {
                    ([<fn_lock_ $fn_name>].get_or_init(|| {
                        let lib = crate::ffi::load_library();
                        match lib.get(concat!($symbol_name, "\0").as_bytes()) {
                            Ok(symbol) => symbol,
                            Err(e) => panic!("Failed to load symbol {}: {:?}", $symbol_name, e),
                        }
                    }))($($arg_name),*)
                }
            }
        }
    };
}

static SK_LIB: OnceLock<Library> = OnceLock::new();

pub unsafe fn load_library() -> &'static Library {
    SK_LIB.get_or_init(|| {
        match unsafe { Library::new(library_filename("StereoKitC")) } {
            Ok(lib) => return lib,
            Err(e) => panic!("Failed to load StereoKitC: {:?}", e),
        }
    })
}
