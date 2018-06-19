extern crate proc_macro2;


extern crate regex;

#[macro_use]
extern crate quote;
use quote::ToTokens;
use proc_macro2::{TokenStream, Span, Ident};
pub use std::{path, env, fs};
pub use std::process::{Command, Stdio};
use std::io::Write;

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

    create_dylib_rs(funcs);

    Ok(())
}

fn create_dylib_rs(funcs: Vec<Def>) {
    let re_name = Regex::new(r"^fn (?P<name>.*?)\(").unwrap();
    let re_par_list = Regex::new(r"(?P<par>\w+): *(?P<ty>[^,)]+)").unwrap();
    let re_no_parm = Regex::new(r"^fn \w+ *\(\)").unwrap();
    let re_ret_val = Regex::new(r"-> *(?P<retTy>.[^{ ]*)").unwrap(); 
    //TODO: I'm breaking on space as well as {, check it to see if there is space in return type like vec

    let mut content = quote! {
        extern crate userProjectLazy as lazy;
    };

    for func in funcs {
        let text = func.sig.expect("Panic! No sig!!").text; //"fn foo() -> f64  {}";//
        let mut parms: Vec<&str> = Vec::new();
        let mut types: Vec<&str> = Vec::new();
        let ret_ts : Option<Ident>;
        println!("{}", text);

        let fn_name = re_name.captures(&text).expect("Panic! no fn_name!!").get(1).unwrap().as_str();
        if !re_no_parm.is_match(&text) { // fn has params
            for caps in re_par_list.captures_iter(&text){
                parms.push(caps.get(1).unwrap().as_str());
                types.push(caps.get(2).unwrap().as_str());
            } 
        }
        match re_ret_val.captures(&text) {
            Some(x) => {
                    let ret_val = x.get(1).unwrap().as_str();
                    ret_ts = Some(Ident::new(ret_val, Span::call_site()));
            },
            None => ret_ts = None,
        }
        let ret_expression = match ret_ts {
            Some(x) => quote! ( -> #x ),
            None => quote! (),
        };
        //let ret_val = re_ret_val.captures(&text).unwrap().get(1).unwrap().as_str();

        println!("fn_name {}\nparms {:?}\ntypes {:?}", fn_name, parms, types);
        //let fn_name = &text.split(" ").next().unwrap().split('(').first();


        let name = proc_macro2::Ident::new(fn_name, Span::call_site());
        let mut parm_list_decl = TokenStream::new();
        let mut parm_list_call = TokenStream::new();

        for (idx, par) in parms.iter().enumerate() {
            let pp = Ident::new(par, Span::call_site());
            let tt = Ident::new(types[idx], Span::call_site());
            quote!(
                #pp: #tt, 
            ).to_tokens(&mut parm_list_decl);

            quote! (
                #pp,   
            ).to_tokens(&mut parm_list_call);

        }
        //let ret : Option<Ident> = Some(Ident::new(ret_val, Span::call_site()));
        //println!("{}",parm_list );

        quote!(
            #[no_mangle]
            pub extern "C" fn #name(#parm_list_decl)   #ret_expression {
                lazy::#name(#parm_list_call)
            }
        ).to_tokens(&mut content);
    } // end of forr on funcs
    println!("{}", content);

    let mut file = match fs::File::create("nvdDylib.rs") {
        Err(oops) => panic! ("couldn't creat file! {}", oops),
        Ok(fl) => fl,
    };

    let output = format!("{}", content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }

}