use std::{env::current_dir, ffi::OsStr, process::Stdio};

use stereokit_rust::tools::build_tools::{compile_hlsl, copy_tree, get_cargo_name};

pub const USAGE: &str = r#"Usage : cargo build_sk_rs [Options] <Output_path>
    Build the project then copy files to <Output_path>
    
    Options:
        --debug                         : Build a debug instead of a release.
        --x64-win-gnu    <path_to_libs> : x86_64-pc-windows-gnu DirectX11 build using 
                                          path_to_libs where some libraries must be 
                                          set (work in progress ...).
        --gl                            : for windows build will use OPENGL instead of 
                                          D3D11.
        --features <feat1, feat2 ...>   : Features of the project to turn on.
        --example  <exe_name>           : If the project has an examples directory, 
                                          will execute the program <exe_name>.
        --shaders <path_to_shaders>     : Use sks shaders from path_to_shaders.
                                          By default, shaders are optimized for
                                          the target platform.
        -h|--help                       : Display help"#;

enum Target {
    Default,
    X86_64WinGnu,
}
fn main() {
    use std::{env, fs, path::PathBuf, process::Command};

    //----First the command line
    let mut output_path_name = "".to_string();
    let mut output_path_already_exists = false;
    let mut build_target = Target::Default;
    let mut win_libs_path_name = "".to_string();
    let mut with_gl = false;
    let mut features = "".to_string();
    let mut feature_list = vec![];
    let mut example = "".to_string();
    let mut example_exe = "".to_string();
    let mut shaders_path_name = "".to_string();
    let mut profile = "--release".to_string();

    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match &arg[..] {
            "build_sk_rs" => {}
            "--debug" => {
                profile = "".to_string(); //--debug is the default
            }
            "--x64-win-gnu" => {
                if let Some(arg_config) = args.next() {
                    if !arg_config.starts_with('-') {
                        build_target = Target::X86_64WinGnu;
                        win_libs_path_name = arg_config;
                    } else {
                        println!("Value specified for --x64-win-gnu must be the path of a directory.");
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("No value specified for parameter --x64-win-gnu.");
                    panic!("{}", USAGE);
                }
            }
            "--gl" => {
                with_gl = true;
            }
            "--features" => {
                features = "--features".to_string();
                for arg_config in args.by_ref() {
                    feature_list.push(arg_config.clone());
                    if !arg_config.ends_with(',') {
                        break;
                    }
                }
            }
            "--example" => {
                example = "--example".to_string();
                if let Some(arg_config) = args.next() {
                    if !arg_config.starts_with('-') {
                        example_exe = arg_config;
                    } else {
                        println!("Value specified for --example must be the name of an executable.");
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("No value specified for parameter --example.");
                    panic!("{}", USAGE);
                }
            }
            "--shaders" => {
                if let Some(arg_config) = args.next() {
                    if !arg_config.starts_with('-') {
                        shaders_path_name = arg_config;
                        let shaders_path = PathBuf::from(&shaders_path_name);
                        if !shaders_path.exists() || !shaders_path.is_dir() {
                            panic!("Arg --shaders: The <path_to_shaders> should be a directory");
                        }
                    } else {
                        println!("Value specified for --shaders must be the path of a directory.");
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("No value specified for parameter --x86_64-win-gnu.");
                    panic!("{}", USAGE);
                }
            }
            "-h" => panic!("{}", USAGE),
            "--help" => panic!("{}", USAGE),
            _ => {
                if arg.starts_with('-') {
                    println!("Unkown argument {}", arg);
                    panic!("{}", USAGE);
                } else if output_path_name.is_empty() {
                    let path = PathBuf::from(&arg);
                    if path.exists() && path.parent().unwrap().is_dir() {
                        output_path_name = arg;
                    } else {
                        println!("Argument {} should be path to an existing directory", arg);
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("Unkown positional argument {}", arg);
                    panic!("{}", USAGE);
                }
            }
        }
    }
    let output_path = PathBuf::from(&output_path_name);
    if let Err(_err) = fs::create_dir(&output_path) {
        output_path_already_exists = true
    };

    if !output_path.exists() || !output_path.is_dir() {
        panic!("You forgot to indicate the <output_path> to create");
    }

    if !output_path.is_dir() {
        panic!("Argument {} should be a valid directory name", output_path_name);
    }

    //----Second the cargo build command
    let mut windows_exe = if cfg!(target_os = "windows") { ".exe" } else { "" };

    let mut cmd = Command::new("cargo");
    cmd.stdout(Stdio::piped()).arg("build");

    if !win_libs_path_name.is_empty() {
        match build_target {
            Target::Default => panic!("Not possible"),
            Target::X86_64WinGnu => {
                windows_exe = ".exe";
                cmd.arg("--target=x86_64-pc-windows-gnu");
            }
        };
        cmd.env("SK_RUST_WIN_GNU_LIBS", &win_libs_path_name);
    }

    if with_gl {
        cmd.env("SK_RUST_WINDOWS_GL", "ON");
    }

    cmd.arg(&profile);

    if !features.is_empty() {
        cmd.arg(&features);
        for feature in feature_list {
            cmd.arg(feature);
        }
    }

    if !example.is_empty() {
        cmd.arg(&example).arg(&example_exe);
    }

    let child = cmd.spawn().expect("failed to run cargo build");
    let output = child.wait_with_output().expect("failed to wait on child");
    println!("{}", String::from_utf8(output.clone().stdout).unwrap_or(format!("{:#?}", output)));
    println!("{}", String::from_utf8(output.clone().stderr).unwrap_or(format!("{:#?}", output)));

    if !output.status.success() {
        panic!("cargo build failed!")
    }

    //----Third the file copy
    if output_path_already_exists {
        println!("Replacing file in {}", output_path_name);
        println!("Warning! Old abandoned files will not be deleted !");
    }

    let mut built_files = PathBuf::from("target");

    if !win_libs_path_name.is_empty() {
        match build_target {
            Target::Default => panic!("Again Impossible!!"),
            Target::X86_64WinGnu => built_files = built_files.join("x86_64-pc-windows-gnu"),
        };
    }
    built_files = built_files.join(profile.strip_prefix("--").unwrap_or_default());

    // 1 - the executable
    let project_id = get_cargo_name().unwrap();
    println!("Project name is {}", project_id);
    let exe_file = if !example.is_empty() {
        built_files.join("examples").join(example_exe + windows_exe)
    } else {
        built_files.join(format!("{}{}", project_id, windows_exe))
    };
    let dest_file_exe = output_path.join(exe_file.file_name().unwrap_or_default());
    println!("Executable is copied from here --> {:?}", exe_file);
    println!("                       to here --> {:?}", dest_file_exe);
    let _lib_exe = fs::copy(exe_file, dest_file_exe).unwrap();

    if !windows_exe.is_empty() {
        // 1-1 - the dlls created
        let dll_file = built_files.join("deps").join("stereokit_rust.dll");
        let dest_file_dll = output_path.join(dll_file.file_name().unwrap_or_default());
        println!("DLL is copied from here --> {:?}", dll_file);
        println!("                to here --> {:?}", dest_file_dll);
        let _lib_dll = fs::copy(dll_file, dest_file_dll).unwrap();

        let c_dll = if !win_libs_path_name.is_empty() { "libStereoKitC.dll" } else { "StereoKitC.dll" };
        let dll_file = built_files.join("deps").join(c_dll);
        let dest_file_dll = output_path.join(dll_file.file_name().unwrap_or_default());
        println!("DLL is copied from here --> {:?}", dll_file);
        println!("                to here --> {:?}", dest_file_dll);
        let _lib_dll = fs::copy(dll_file, dest_file_dll).unwrap();

        let copy_extensions = [OsStr::new("dll")];
        if !win_libs_path_name.is_empty() {
            // 1-2 - the dlls mingw ask for
            let libs_path = PathBuf::from(win_libs_path_name);
            for entry in libs_path.read_dir().expect("Libs path is not a valid directory!").flatten() {
                let file = entry.path();
                if file.is_file() {
                    if let Some(extension) = file.extension() {
                        if copy_extensions.contains(&extension) {
                            println!("Mingw Dll to copy {:?}", file);
                            let dest_file_dll = output_path.join(file.file_name().unwrap_or_default());
                            let _lib_dll = fs::copy(file, dest_file_dll).unwrap();
                        }
                    }
                }
            }
        }
    }

    // 2 - the assets
    let from_assets = PathBuf::from("assets");
    let to_asset = output_path.join("assets");
    copy_tree(from_assets, to_asset).unwrap();

    // 3 - the shaders
    let target_shaders_dir = output_path.join("assets").join("shaders");
    if shaders_path_name.is_empty() {
        let target = if windows_exe.is_empty() {
            "e"
        } else if with_gl {
            "g"
        } else {
            "x"
        };
        compile_hlsl(current_dir().unwrap(), Some(target_shaders_dir), &["-f", "-t", target, "-sw"]).unwrap();
    } else {
        let shaders_path = PathBuf::from(shaders_path_name);
        for entry in shaders_path.read_dir().expect("shaders_path is not a valid directory!").flatten() {
            let file = entry.path();
            if file.is_file() {
                println!("Shader to copy {:?}", file);
                let dest_file_dll = target_shaders_dir.join(file.file_name().unwrap_or_default());
                let _lib_dll = fs::copy(file, dest_file_dll).unwrap();
            }
        }
    }
}
