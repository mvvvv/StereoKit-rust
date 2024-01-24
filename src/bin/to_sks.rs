use std::{ ffi::OsStr, io, path::PathBuf, process::Command};

fn main() {
    is_input_file_outdated().unwrap();
}

fn is_input_file_outdated() -> Result<bool, io::Error> {
    let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut skshaderc = project_dir.clone();
    skshaderc.push(r"StereoKit");
    skshaderc.push(r"tools");
    if cfg!(windows) {
        skshaderc.push("skshaderc.exe");
    } else {
        skshaderc.push("skshaderc");
    }

    let mut shaders_path = project_dir.clone();
    shaders_path.push("assets");
    shaders_path.push("shaders");

    println!("Shaders path {:?}",shaders_path);

    let command = OsStr::new(skshaderc.as_os_str());
    let excluded_extensions = [OsStr::new("sks"), OsStr::new("txt"), OsStr::new("md")];
    if let Ok(entries) = shaders_path.read_dir() {
        for entry in entries {
            let file = entry?.path();
            if file.is_file() {
                if let Some(extension) = file.extension() {
                    if !excluded_extensions.contains(&extension) {

                        println!("shader file : {:?}",file);
                        let output = Command::new(command)
                            .args(&[file])
                            .output()
                            .expect(format!("failed to run {}", command.to_str().unwrap()).as_str());
                        println!("{}", String::from_utf8(output.stdout).unwrap());
                    }
                }
            }
        }
    }
    Ok(true)
}
