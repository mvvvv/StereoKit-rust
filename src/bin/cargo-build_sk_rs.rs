use std::{ffi::OsStr, fs::create_dir, process::Stdio};

use stereokit_rust::tools::{
    build_tools::{compile_hlsl, copy_tree, get_cargo_name},
    os_api::{get_assets_dir, get_shaders_sks_dir},
};

pub const USAGE: &str = r#"Usage : cargo build_sk_rs [Options] <Output_path>
    Build the project then copy files to <Output_path>
    
    Options:
        --debug                         : Build a debug instead of a release.
        --x64-win-gnu    <path_to_libs> : x86_64-pc-windows-gnu DirectX11 build using 
                                          path_to_libs where some libraries must be 
                                          set (libgcc_eh.a).
        --x64-linux                     : On Linux, x86_64-unknown-linux-gnu build
        --aarch64-linux                 : On Linux, aarch64-unknown-linux-gnu build
        --gl                            : For windows, build will use OPENGL instead of 
                                          D3D11.
        --features <feat1, feat2 ...>   : Features of the project to turn on.
        --example  <exe_name>           : If the project has an examples directory, 
                                          will execute the program <exe_name>.
        --bin  <exe_name>               : If the project has a bin directory, 
                                          will execute the program <exe_name>.
        --shaders <path_to_shaders>     : Use sks shaders from path_to_shaders.
                                          By default, shaders are optimized for
                                          the target platform.
        -h|--help                       : Display help
        
        
    If you want DLL instead of static link use the feature skc-in-dll"#;

enum Target {
    Default,
    X86_64WinGnu,
    X86_64Linux,
    Aarch64Linux,
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
    let mut bin = "".to_string();
    let mut bin_exe = "".to_string();
    let mut shaders_path_name = "".to_string();
    let mut profile = "--release".to_string();

    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match &arg[..] {
            "build_sk_rs" => {}
            "--debug" => {
                profile = "--debug".to_string();
            }
            "--x64-win-gnu" => {
                if let Some(arg_config) = args.next() {
                    if !arg_config.starts_with('-') {
                        build_target = Target::X86_64WinGnu;
                        win_libs_path_name = arg_config;
                        let win_libs_path = PathBuf::from(&win_libs_path_name);
                        if !win_libs_path.is_dir() {
                            println!("Argument {} should be a valid directory name", win_libs_path_name);
                            panic!("{}", USAGE);
                        }
                    } else {
                        println!("Value specified for --x64-win-gnu must be the path of a directory.");
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("No value specified for parameter --x64-win-gnu.");
                    panic!("{}", USAGE);
                }
            }
            "--x64-linux" => {
                build_target = Target::X86_64Linux;
            }
            "--aarch64-linux" => {
                build_target = Target::Aarch64Linux;
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
            "--bin" => {
                bin = "--bin".to_string();
                if let Some(arg_config) = args.next() {
                    if !arg_config.starts_with('-') {
                        bin_exe = arg_config;
                    } else {
                        println!("Value specified for --bin must be the name of an executable.");
                        panic!("{}", USAGE);
                    }
                } else {
                    println!("No value specified for parameter --bin.");
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
        println!("You forgot to indicate the <output_path> to create");
        panic!("{}", USAGE);
    }
    if !output_path.is_dir() {
        println!("Argument {} should be a valid directory name", output_path_name);
        panic!("{}", USAGE);
    }

    if !example.is_empty() && !bin.is_empty() {
        println!("You cannot specify both --example and --bin");
        panic!("{}", USAGE);
    }

    //----Second the cargo build command
    let mut windows_exe = if cfg!(target_os = "windows") { ".exe" } else { "" };

    let mut cmd = Command::new("cargo");
    cmd.stdout(Stdio::piped()).arg("build");

    match build_target {
        Target::Default => {}
        Target::X86_64WinGnu => {
            cmd.env("SK_RUST_WIN_GNU_LIBS", &win_libs_path_name);
            windows_exe = ".exe";
            cmd.arg("--target=x86_64-pc-windows-gnu");
        }
        Target::X86_64Linux => {
            cmd.arg("--target=x86_64-unknown-linux-gnu");
        }
        Target::Aarch64Linux => {
            cmd.arg("--target=aarch64-unknown-linux-gnu");
        }
    };

    if with_gl {
        cmd.env("SK_RUST_WINDOWS_GL", "ON");
    }

    if profile != "--debug" {
        cmd.arg(&profile);
    }

    if !features.is_empty() {
        cmd.arg(&features);
        for feature in feature_list {
            cmd.arg(feature);
        }
    }

    if !example.is_empty() {
        cmd.arg(&example).arg(&example_exe);
    }

    if !bin.is_empty() {
        cmd.arg(&bin).arg(&bin_exe);
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

    match build_target {
        Target::Default => {}
        Target::X86_64WinGnu => built_files = built_files.join("x86_64-pc-windows-gnu"),
        Target::X86_64Linux => built_files = built_files.join("x86_64-unknown-linux-gnu"),
        Target::Aarch64Linux => built_files = built_files.join("aarch64-unknown-linux-gnu"),
    };
    built_files = built_files.join(profile.strip_prefix("--").unwrap_or_default());

    // 1 - the executable
    let project_id = get_cargo_name().unwrap();
    println!("Project name is {}", project_id);
    let exe_file = if !example.is_empty() {
        built_files.join("examples").join(example_exe + windows_exe)
    } else if !bin.is_empty() {
        built_files.join(bin_exe + windows_exe)
    } else {
        built_files.join(format!("{}{}", project_id, windows_exe))
    };
    let dest_file_exe = output_path.join(exe_file.file_name().unwrap_or_default());
    println!("Executable is copied from here --> {:?}", exe_file);
    println!("                      to there --> {:?}", dest_file_exe);
    let _lib_exe = fs::copy(&exe_file, dest_file_exe).unwrap();

    if !windows_exe.is_empty() {
        let pdb_file_name: String = exe_file
            .to_str()
            .expect("exe str name problem!!")
            .strip_suffix(".exe")
            .expect("exe name doesn't finish with .exe")
            .to_string()
            + ".pdb";
        let pdb_file = PathBuf::from(&pdb_file_name);
        if pdb_file.is_file() {
            let dest_file_pdb = output_path.join(pdb_file.file_name().expect("No filename for PDB"));
            println!("Exe's PDB  is copied from here --> {:?}", pdb_file);
            println!("                      to there --> {:?}", dest_file_pdb);
            let _lib_pdb = fs::copy(pdb_file, dest_file_pdb).unwrap();
        }
        // 1-1 - the dlls created
        let c_dll = if !win_libs_path_name.is_empty() { "libStereoKitC.dll" } else { "StereoKitC.dll" };
        let dll_file = built_files.join("deps").join(c_dll);
        if dll_file.is_file() {
            let dest_file_dll = output_path.join(dll_file.file_name().unwrap_or_default());
            println!("DLL is copied from here --> {:?}", dll_file);
            println!("               to there --> {:?}", dest_file_dll);
            let _lib_dll = fs::copy(dll_file, dest_file_dll).unwrap();

            let c_pdb = if !win_libs_path_name.is_empty() { "libStereoKitC.pdb" } else { "StereoKitC.pdb" };
            let pdb_file = built_files.join("deps").join(c_pdb);
            if pdb_file.is_file() {
                let dest_file_pdb = output_path.join(pdb_file.file_name().unwrap_or_default());
                println!("PDB is copied from here --> {:?}", pdb_file);
                println!("               to there --> {:?}", dest_file_pdb);
                let _lib_pdb = fs::copy(pdb_file, dest_file_pdb).unwrap();
            }

            // let dll_file = built_files.join("deps").join("stereokit_rust.dll");
            // if dll_file.is_file() {
            //     let dest_file_dll = output_path.join(dll_file.file_name().unwrap_or_default());
            //     println!("DLL is copied from here --> {:?}", dll_file);
            //     println!("               to there --> {:?}", dest_file_dll);
            //     let _lib_dll = fs::copy(dll_file, dest_file_dll).unwrap();
            // }

            let copy_extensions = [OsStr::new("dll")];
            if !win_libs_path_name.is_empty() {
                // 1-2 - the dlls mingw ask for
                let libs_path = PathBuf::from(win_libs_path_name.clone());
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
    }

    let real_current_dir = PathBuf::from(".");

    // 2 - the assets
    let from_assets = PathBuf::from(get_assets_dir());
    let to_asset = output_path.join(get_assets_dir());
    if from_assets.exists() {
        println!("Copying assets from {:?} to {:?}", from_assets, to_asset);
        copy_tree(from_assets, to_asset.clone()).unwrap();
    } else {
        println!(
            "Assets directory not found! {from_assets:?}\n---The name of the directory may be change with SK_RUST_ASSET_DIR"
        )
    }
    // 3 - the shaders
    let mut with_wine = false;
    let target_shaders_dir = to_asset.join(get_shaders_sks_dir());
    if !target_shaders_dir.exists() {
        create_dir(&target_shaders_dir).expect("Unable to create shaders directory");
    }
    if shaders_path_name.is_empty() {
        let target = if windows_exe.is_empty() {
            "e"
        } else if with_gl {
            "g"
        } else {
            if !cfg!(windows) && !win_libs_path_name.is_empty() {
                with_wine = true;
            }
            "x"
        };

        compile_hlsl(real_current_dir, Some(target_shaders_dir), &["-t", target, "-sw"], with_wine).unwrap();
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
