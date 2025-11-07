use std::env;
use std::path::PathBuf;

fn main() {
    system_deps::Config::new().probe().unwrap();
}
