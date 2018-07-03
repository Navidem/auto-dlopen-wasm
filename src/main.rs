extern crate auto_dlopen_wasm as dl;

use std::{path, env};

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        panic!("\n\nPlease provide options:\n-standalone <path>: run the standalone auto-dlopen\n
                -scaffold <path>: generate build scripts and Cargo.toml for \"native\" binary\n
                -scaffold-wasm <path>: generate build scripts and Cargo.toml for \"wasm\" binary")
    } 
    let option = &args[1];
    let path_string = &args[2];
    let path = path::Path::new(path_string);
    println!("Wroking path: {:?}", path);
    match option.as_ref() {
        "-standalone" => {
            let funcs = dl::run_analysis(&path.join("lazy"), "crate")?;

            let token_stream = dl::create_func_tokens(funcs);
            dl::write_dylib(&token_stream, path);
            dl::write_client(&token_stream, path);
        },
        "-scaffold" => {
            dl::generate_build_scripts(path, "crate");
        }
        "-scaffold-wasm" => {   //expects to see lazy module in src/lazy/mod.rs
            let funcs = dl::run_analysis(&path.join("src/lazy"), "module")?;
            println!("{:?}", funcs );
        }
        _ => panic!("\n\ninvalid option!\n-standalone <path>: run the standalone auto-dlopen\n
                -scaffold <path>: generate build scripts and Cargo.toml for \"native\" binary\n
                -scaffold-wasm <path>: generate build scripts and Cargo.toml for \"wasm\" binary")
        }
    Ok(())
}