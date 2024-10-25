use std::env::{args, current_dir};

use stereokit_rust::tools::build_tools::compile_hlsl;

pub const USAGE: &str = r#"Usage : cargo compile_sks [Options] <Output_path>
    Compile the HLSL files in shader_src to assets/shaders
    
    Options:
        --options      : skshaderc's options except -o 
        -h|--help      : Display help"#;

fn main() {
    //----First the command line
    let mut with_option = false;
    let mut options = vec![];

    let args = args().skip(1);

    for arg in args {
        match &arg[..] {
            "compile_sks" => {}
            "--options" => {
                with_option = true;
            }
            "-h" => panic!("{}", USAGE),
            "--help" => panic!("{}", USAGE),
            _ => {
                if with_option {
                    if arg == "-o" {
                        println!("-o is not accepted");
                        panic!("{}", USAGE);
                    }
                    options.push(arg);
                } else {
                    println!("Unkown argument {}", arg);
                    panic!("{}", USAGE);
                }
            }
        }
    }

    let project_dir = current_dir().unwrap();

    let options_str = Vec::from_iter(options.iter().map(String::as_str));

    compile_hlsl(project_dir, None, options_str.as_slice()).unwrap();
}
