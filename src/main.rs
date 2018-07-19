extern crate auto_dlopen_wasm as dl;

use std::{path, env};

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        panic!("\n\nPlease provide options: {:?}\n-standalone <path>: run the standalone auto-dlopen\n
-scaffold <path>: generate build scripts and Cargo.toml for \"native\" binary\n
-scaffold-wasm <path>: generate build scripts and Cargo.toml for \"wasm\" binary", args)
    } 
    let option = &args[1];
    let path_string = &args[2];
    let path = path::Path::new(path_string);
    println!("Wroking path: {:?}", path);
    match option.as_ref() {
        "-standalone" => {
            let funcs = dl::run_analysis(&path.join("lazy"), "crate")?;

            let token_stream = dl::create_func_tokens(funcs);
            dl::write_dylib(&token_stream, path, false);
            dl::write_client(&token_stream, path, false);
        },
        "-scaffold" => {
            dl::generate_build_scripts(path, "crate", false);
        }
        "-scaffold-wasm" => {   //expects to see lazy module in src/lazy/mod.rs
            dl::generate_build_scripts(path, "crate", true);
        }
        _ => panic!("\n\ninvalid option!\n-standalone <path>: run the standalone auto-dlopen\n
                -scaffold <path>: generate build scripts and Cargo.toml for \"native\" binary\n
                -scaffold-wasm <path>: generate build scripts and Cargo.toml for \"wasm\" binary")
        }
    Ok(())
}