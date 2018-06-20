extern crate auto_dlopen as dl;

use std::{path, env};

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    let mut path = path::Path::new(".");
    if args.len() > 1 {
        path = path::Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);
    let funcs = dl::run_analysis(&path.join("lazy"))?;

    let token_stream = dl::create_func_tokens(funcs);
    dl::write_dylib(&token_stream, path);
    dl::write_client(&token_stream, path);

    Ok(())
}
