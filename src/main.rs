extern crate rls;

use std::{path, env};
use rls::start_analysis;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut path = path::Path::new(".");
    if args.len() > 1 {
        path = path::Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);
    start_analysis(path);

}