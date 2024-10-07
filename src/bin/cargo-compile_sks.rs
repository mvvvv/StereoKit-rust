use std::{
    env::current_dir,
    ffi::OsStr,
    fs::create_dir,
    io,
    path::PathBuf,
    process::{exit, Command},
};

fn main() {
    is_input_file_outdated().unwrap();
}

fn is_input_file_outdated() -> Result<bool, io::Error> {
    let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_dir = current_dir().unwrap();

    let target_os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        ""
    };
    let target_arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        ""
    };
    let exe_type = target_os.to_string() + "_" + target_arch;

    let mut skshaderc = bin_dir.clone();
    skshaderc.push(r"StereoKit");
    skshaderc.push(r"tools");
    skshaderc.push(r"skshaderc");
    skshaderc.push(exe_type);
    if cfg!(windows) {
        skshaderc.push("skshaderc.exe");
    } else {
        skshaderc.push("skshaderc");
    }
    let mut shaders_source_path = project_dir.clone();
    shaders_source_path.push("shaders_src");

    if !shaders_source_path.exists() || !shaders_source_path.is_dir() {
        println!("Current directory do not contain shaders_src directory");
        println!("Abort!");
        exit(1);
    }

    let mut shaders_path = project_dir.clone();
    shaders_path.push("assets");
    if !shaders_path.exists() || !shaders_path.is_dir() {
        println!("Current directory do not contain assets directory");
        println!("Abort!");
        exit(2);
    }
    shaders_path.push("shaders");
    if !shaders_path.exists() || !shaders_path.is_dir() {
        if let Err(e) = create_dir(&shaders_path) {
            println!("Unable to create the directory assets/shaders inside the current directory");
            println!("Error : {:?}", e);
            println!("Abort!");
            exit(3);
        }
    }

    let mut shaders_include = bin_dir.clone();
    shaders_include.push("StereoKit");
    shaders_include.push("tools");
    shaders_include.push("include");

    println!("skshaderc executable used :  {:?}", &skshaderc);
    println!("Shaders compiled there : {:?}", &shaders_path);

    let command = OsStr::new(skshaderc.as_os_str());
    let excluded_extensions = [OsStr::new("hlsli"), OsStr::new("sks"), OsStr::new("txt"), OsStr::new("md")];
    if let Ok(entries) = shaders_source_path.read_dir() {
        for entry in entries {
            let file = entry?.path();
            if file.is_file() {
                if let Some(extension) = file.extension() {
                    if !excluded_extensions.contains(&extension) {
                        //println!("shader file : {:?}", file);
                        let output = Command::new(command)
                            .arg(file)
                            .arg("-i")
                            .arg(&shaders_path)
                            .arg("-i")
                            .arg(&shaders_include)
                            .arg("-o")
                            .arg(&shaders_path)
                            .output()
                            .expect("failed to run shader compiler");
                        println!("{}", String::from_utf8(output.clone().stdout).unwrap_or(format!("{:#?}", output)));
                    }
                }
            }
        }
    }
    Ok(true)
}
