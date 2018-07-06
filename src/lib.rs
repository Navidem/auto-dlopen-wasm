extern crate proc_macro2;
extern crate  syn;
extern crate regex;
extern crate byteorder;

#[macro_use]
extern crate quote;
use quote::ToTokens;
use proc_macro2::{TokenStream, Span, Ident};

pub use std::{path, env, fs};
pub use std::process::{Command, Stdio};
use std::io::{Write, Read};
use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::OpenOptions;
mod rls;

pub use rls::run_analysis;
use rls::rls_analysis::Def as Def;
use regex::Regex;

#[derive(Debug)]
pub struct FuncTokens {
    name: Ident,
    param_list_decl: TokenStream,
    param_list_call: TokenStream,
    ret_expression: TokenStream,
}


pub fn create_func_tokens(funcs: Vec<Def>) -> Vec<FuncTokens> {
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
            let ty = types[idx];
            let tt = match ty.find('<') { //if we have like Vec<i32>
                Some(loc) => {
                    let split: Vec<&str> = ty.split('<').collect();
                    let out = Ident::new(split[0], Span::call_site());
                    let inn = Ident::new(&split[1][..split.len()-1], Span::call_site());
                    quote!(#out<#inn>)
                }
                None => {
                    let ident = Ident::new(ty, Span::call_site());
                    quote! (#ident)
                }
            };
            // let tt = Ident::new(types[idx], Span::call_site());
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


    } // end of for on funcs
    func_ts_list
   // write_dylib(&func_ts_list, path);
   // write_client(&func_ts_list, path);

 
}

pub fn write_dylib (func_list: &Vec<FuncTokens>, path: &path::Path) {
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
    format_src(src_file);
    println!("dylib/ contents successfully created!" );

}


pub fn write_client(func_list: &Vec<FuncTokens>, path: &path::Path) {
    //let func = func_list.get(0).unwrap();
    let mut trait_ts = TokenStream::new();
    let mut impl_ts = TokenStream::new();

    for func in func_list {
        let name = &func.name;
        let literal_name = proc_macro2::Literal::byte_string(format!("{}",name).as_bytes()); //required for lib.get(b"symbol")
        let param_list_decl = &func.param_list_decl;
        let param_list_call = &func.param_list_call;
        let ret_expression = &func.ret_expression;

        quote! (
            fn #name(&self, #param_list_decl) #ret_expression;
        ).to_tokens(&mut trait_ts);

        quote! (
            fn #name(&self, #param_list_decl) #ret_expression { 
                let lib = &self.dylib;
                unsafe{
                    let func: Symbol<unsafe extern fn (#param_list_decl) #ret_expression> = lib.get(#literal_name).unwrap();
                    func(#param_list_call)
                }
            }
        ).to_tokens(&mut impl_ts);
    }


    let content = quote! {
        extern crate libloading;
        use libloading::{Library,Symbol};

        pub trait LazyDylibTrait {
            #trait_ts
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
        impl LazyDylibTrait for LazyDylib {
            #impl_ts
        }
    };


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
    format_src(src_file);

    println!("client/ contents successfully created!" );

}

pub fn generate_client(path: &path::Path, mode: &str) -> Result<(), Box<std::error::Error>> {
    println!("Called generate_client");
    let funcs = run_analysis(&path.join("lazy"), mode)?;
    let token_stream = create_func_tokens(funcs);
    write_client(&token_stream, path);
    Ok(())
}

pub fn generate_dylib (path: &path::Path, mode: &str) -> Result<(), Box<std::error::Error>> {
    println!("Called generate_dylib");
    let funcs = run_analysis(&path.join("lazy"), mode)?;
    let token_stream = create_func_tokens(funcs);
    write_dylib(&token_stream, path);
    Ok(())
}

pub fn generate_build_scripts(path: &path::Path, mode: &str){
    //dylib: build.rs & cargo
    write_build_rs(path, "dylib", mode);
    let dylib_cargo_content = "[package]
name = \"userProjectDylib\"
version = \"0.0.1\"

[lib]
path = \"./src/lib.rs\"
crate-type = [\"cdylib\"]

[dependencies]
userProjectLazy = {path = \"../lazy\"}
[build-dependencies]
auto-dlopen-wasm = {path = \"../../auto-dlopen-wasm\"}";

    let cargo_file = path.join("dylib/Cargo.toml");
    let mut file = match fs::File::create(&cargo_file) {
        Err(oops) => panic! ("couldn't create dylib/Cargo.toml file! {} {:?}", oops, cargo_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", dylib_cargo_content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }


    //client cargo and build.rs
    write_build_rs(path, "client", mode);
    let client_cargo_content = "[package]
name = \"userProjectClient\"
version = \"0.0.1\"

[lib]
path = \"./src/lib.rs\"

[dependencies]
libloading = \"0.5.0\"
[build-dependencies]
auto-dlopen-wasm = {path = \"../../auto-dlopen-wasm\"}";

    let cargo_file = path.join("client/Cargo.toml");
    let mut file = match fs::File::create(&cargo_file) {
        Err(oops) => panic! ("couldn't create Cargo.toml file! {} {:?}", oops, cargo_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", client_cargo_content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }

}

fn write_build_rs(path: &path::Path, dest: &str, mode: &str) {
    let path_string = String::from(path.to_str().unwrap());
    let method_name = Ident::new(&("generate_".to_owned()+dest), Span::call_site());
    let err_msg = "Error! could not generate ".to_owned() + dest;
    let content = quote!{
        extern crate auto_dlopen_wasm as dlopen;
        use std::path;

        fn main() {
            let top_level_path = path::Path::new(#path_string);
            match dlopen::#method_name(top_level_path, #mode) {
                Ok(_) => (),
                _ => panic!(#err_msg)
            }
        }
    };
    let src_file = path.join(dest).join("build.rs");
    let mut file = match fs::File::create(&src_file) {
        Err(oops) => panic! ("couldn't create build.rs file! {} {:?}", oops, src_file),
        Ok(fl) => fl,
    };

    let output = format!("{}", content);
    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }
    format_src(src_file);

}

fn format_src (src_file: path::PathBuf){
    let mut command = Command::new("rustfmt");
    command.arg(src_file);
    match command.spawn(){
        Err(oops) => panic!("rustfmt faild! {}", oops),
        Ok(_) => (),
    }

}
pub fn generate_build_scripts_wasm(path: &path::Path) -> Result<(), Box<std::error::Error>> {
    let funcs = run_analysis(&path, "module")?;
    let token_stream = create_func_tokens(funcs);
    let mut temp_content = vec![];
    let mut static_content_len = 4;
    //content: (num of funcs)|(func1 name len)|....
    temp_content.write_u32::<LittleEndian>(token_stream.len() as u32).unwrap();
    for func in token_stream{
        let name_string = format!("{}", func.name);
        let name_len = name_string.len() as u32;
        static_content_len += 4 + name_len; //len = size literal and the actual string
        temp_content.write_u32::<LittleEndian>(name_len).unwrap();
        temp_content.extend_from_slice( name_string.as_bytes());

    }
    let static_content = syn::LitByteStr::new(&temp_content, Span::call_site());

    let src_file = path.join("src/lib.rs");
    let mut file = match OpenOptions::new().read(true).open(&src_file) {
        Err(oops) => panic!("cannot open src/lib.rs {}", oops),
        Ok(fl) => fl,
    };

    let mut original_content = String::new();
    file.read_to_string(&mut original_content)?;
    println!("{}", original_content);
    let custom_section_content = quote!{
        #![feature(wasm_custom_section)]
        #[wasm_custom_section = "_lazy_wasm_"]
        const WASM_CUSTOM_SECTION: [u8; #static_content_len as usize] = *#static_content; 

    };

    let output = format!("{}\n\n", custom_section_content) + &original_content;
    // file.seek(SeekFrom::Start(0))?;
    //file.truncate();
    drop(file);
    let mut file = match OpenOptions::new().truncate(true).write(true).open(&src_file) {
        Err(oops) => panic!("cannot open src/lib.rs {}", oops),
        Ok(fl) => fl,
    };

    match file.write_all(output.as_bytes()) {
        Err(oops) => panic!("cannot write into file {}", oops),
        Ok(_) => (),
    }
    format_src(src_file);
    println!("src/lib.rs successfully writen for wasm_custom_section");
    Ok(())

}