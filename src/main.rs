
pub use std::{path, env};
pub use std::process::{Command, Stdio};

mod rls;


use rls::run_analysis;

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    let mut path = path::Path::new(".");
    if args.len() > 1 {
        path = path::Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);
    let funcs = run_analysis(path)?;

    for fun in funcs {
        println!("Got this {:?}", fun.sig);
    }


    Ok(())
}