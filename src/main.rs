extern crate proc_macro2;


extern crate regex;

#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;
pub use std::{path, env};
pub use std::process::{Command, Stdio};

mod rls;

use rls::run_analysis;
use rls::rls_analysis::Def as Def;
use regex::{Regex, RegexSet};

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    let mut path = path::Path::new(".");
    if args.len() > 1 {
        path = path::Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);
    let funcs = run_analysis(path)?;

    create_client_rs(funcs);

    Ok(())
}

fn create_client_rs(funcs: Vec<Def>) {
    let re_name = Regex::new(r"^fn (?P<name>.*?)\(").unwrap();
    let re_par_list = Regex::new(r"(?P<par>\w+): *(?P<ty>[^,)]+)").unwrap();
    let re_no_parm = Regex::new(r"^fn \w+ *\(\)").unwrap();
    let mut _tokens = TokenStream::new();

    for func in funcs {
        let text = func.sig.expect("Panic! No sig!!").text;
        let mut parms: Vec<&str> = Vec::new();
        let mut types: Vec<&str> = Vec::new();
        println!("{}", text);

        let fn_name = re_name.captures(&text).expect("Panic! no fn_name!!").get(1).unwrap().as_str();
        if !re_no_parm.is_match(&text) { // fn has params
            for caps in re_par_list.captures_iter(&text){
                parms.push(caps.get(1).unwrap().as_str());
                types.push(caps.get(2).unwrap().as_str());
            } 
        }
        //let parms = par_list.split(',').;

        println!("fn_name {}\nparms {:?}\ntypes {:?}", fn_name, parms, types);
        //let fn_name = &text.split(" ").next().unwrap().split('(').first();

    }
}