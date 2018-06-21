extern crate auto_dlopen as dl;

use std::{path, env};

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        panic!("Please provide options:\n-standalone <path>: run the standalone auto-dlopen\n
                -scaffolding <path>: generate huild scripts and Cargo.toml")
    } 
    let option = &args[1];
    let path_string = &args[2];
    let path = path::Path::new(path_string);
    println!("Wroking path: {:?}", path);
    match option.as_ref() {
        "-standalone" => {
            let funcs = dl::run_analysis(&path.join("lazy"))?;

            let token_stream = dl::create_func_tokens(funcs);
            dl::write_dylib(&token_stream, path);
            dl::write_client(&token_stream, path);
        },
        "-scaffolding" => {
            dl::generate_build_scripts(path);
        }
        _ => panic!("invalid option!\n-standalone <path>: run the standalone auto-dlopen\n
                -scaffolding <path>: generate huild scripts and Cargo.toml")
        }
    Ok(())
}