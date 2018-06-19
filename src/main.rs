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
use regex::Regex;

struct FuncTokens {
    name: Ident,
    param_list_decl: TokenStream,
    param_list_call: TokenStream,
    ret_expression: TokenStream,
}

fn main() -> Result<(), Box<std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    let mut path = path::Path::new(".");
    if args.len() > 1 {
        path = path::Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);
    let funcs = run_analysis(&path.join("lazy"))?;

    create_func_tokens(funcs, path);

    Ok(())
}

fn create_func_tokens(funcs: Vec<Def>, path: &path::Path) {
    let re_name = Regex::new(r"^fn (?P<name>.*?)\(").unwrap();
    let re_par_list = Regex::new(r"(?P<par>\w+): *(?P<ty>[^,)]+)").unwrap();
    let re_no_parm = Regex::new(r"^fn \w+ *\(\)").unwrap();
    let re_ret_val = Regex::new(r"-> *(?P<retTy>.[^{ ]*)").unwrap(); 
    //TODO: I'm breaking on space as well as {, check it to see if there is space in return type like vec
    let mut func_ts_list: Vec<FuncTokens> = Vec::new();

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

        let func_ts = FuncTokens{
            name: name, 
            param_list_call: parm_list_call, 
            param_list_decl: parm_list_decl,
            ret_expression: ret_expression
        };
        func_ts_list.push(func_ts);


    } // end of forr on funcs

    write_dylib(&func_ts_list, path);
    write_client(&func_ts_list, path);

 
}

fn write_dylib (func_list: &Vec<FuncTokens>, path: &path::Path) {
    let mut content = quote! {
        extern crate userProjectLazy as lazy;
    };

    for func in func_list{
        let name = &func.name;
        let param_list_decl = &func.param_list_decl;
        let param_list_call = &func.param_list_call;
        let ret_expression = &func.ret_expression;
        
        quote!(
            #[no_mangle]
            pub extern "C" fn #name(#param_list_decl)   #ret_expression {
                lazy::#name(#param_list_call)
            }
        ).to_tokens(&mut content);
    }
    println!("{}", content);

   match fs::create_dir_all(path.join("dylib/src")) {
        Err(oops) => panic!("Couldn't create dlib/ {}", oops),
        Ok(_) => (),
    }

    let src_file = path.join("dylib/src/lib.rs");
    let mut file = match fs::File::create(&src_file) {
        Err(oops) => panic! ("couldn't create src/lib.rs file! {} {:?}", oops, src_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }

    let cargo_content = "[package]
name = \"userProjectDylib\"
version = \"0.0.1\"

[lib]
path = \"./src/lib.rs\"
crate-type = [\"cdylib\"]

[dependencies]
userProjectLazy = {path = \"../lazy\"}";

    let cargo_file = path.join("dylib/Cargo.toml");
    let mut file = match fs::File::create(&cargo_file) {
        Err(oops) => panic! ("couldn't create Cargo.toml file! {} {:?}", oops, cargo_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", cargo_content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }
    println!("dylib/ contents successfully created!" );

}

// impl ToTokens for [u8] {
//     fn to_tokens(&self, tokens: &mut TokenStream){
//         quote! (
//             self.as_bytes()
//         ).to_tokens(&mut tokens)
//     }
// }

fn write_client(func_list: &Vec<FuncTokens>, path: &path::Path) {
    let func = func_list.get(0).unwrap();

    let name = &func.name;
    let literal_name = proc_macro2::Literal::byte_string(format!("{}",name).as_bytes()); //required for lib.get(b"symbol")
    let param_list_decl = &func.param_list_decl;
    let param_list_call = &func.param_list_call;
    let ret_expression = &func.ret_expression;

    let mut content = quote! {
        extern crate libloading;
        use libloading::{Library,Symbol};

        pub trait LazyDylibTrait {
            fn #name(&self, #param_list_decl) #ret_expression;
        }

        pub struct LazyDylib {
            dylib: Library,
        }

        impl LazyDylib { 
            pub fn open(path: &std::path::Path)  -> Result<Self, Box<std::error::Error>> {
            let loaded_lib = Library::new(path)?;
            Ok(LazyDylib{dylib: loaded_lib})

            }
        }
    };
    quote!(
        impl LazyDylibTrait for LazyDylib {
            fn #name(&self, #param_list_decl) #ret_expression {  
                let lib = &self.dylib;
                unsafe{
                    let func: Symbol<unsafe extern fn (#param_list_decl) #ret_expression> = lib.get(#literal_name).unwrap();
                    func(#param_list_call)
                }
            }
        }).to_tokens(&mut content);

    println!("{}", content);

   match fs::create_dir_all(path.join("client/src")) {
        Err(oops) => panic!("Couldn't create client/ {}", oops),
        Ok(_) => (),
    }

    let src_file = path.join("client/src/lib.rs");
    let mut file = match fs::File::create(&src_file) {
        Err(oops) => panic! ("couldn't create src/lib.rs file! {} {:?}", oops, src_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }
    let cargo_content = "[package]
name = \"userProjectClient\"
version = \"0.0.1\"

[lib]
path = \"./src/lib.rs\"

[dependencies]
libloading = \"0.5.0\"";

    let cargo_file = path.join("client/Cargo.toml");
    let mut file = match fs::File::create(&cargo_file) {
        Err(oops) => panic! ("couldn't create Cargo.toml file! {} {:?}", oops, cargo_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", cargo_content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }

println!("client/ contents successfully created!" );

}