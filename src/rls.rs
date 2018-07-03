pub extern crate rls_analysis;
extern crate rls_data; 
extern crate serde_json;

use self::rls_analysis::{AnalysisHost, DefKind};
use self::rls_data::config::Config as AnalysisConfig;

use std;
use std::path;
use std::process::{Command, Stdio};
use std::str::FromStr;

pub fn run_analysis(path: &path::Path, mode: &str) -> Result< Vec<rls_analysis::Def>, Box<std::error::Error> >{
    let analysis = rls_analysis::AnalysisHost::new(rls_analysis::Target::Debug);
    let mut func_list: Vec<rls_analysis::Def> = Vec::new();
    //path_prefix: Cargo's working directory and will contain the target directory
    //base_dir: is the root of the whole workspace
    println!("rls_analysis on {:?}", path);
    generate_analysis_files(path)?;  // necessary to create the save-analysis dir
    analysis.reload(path, path)?;
    let mut roots = analysis.def_roots()?;
    roots.sort_unstable_by(|(_, name1), (_, name2)| name1.cmp(name2));
    let look_for: String;
    match mode {    //TODO: replace the hardcoded crate/module names
        "crate" => look_for = String::from_str("userProjectLazy")?,
        "module" => look_for = String::from_str("lazy")?,
        _ => panic!{"invalid rls_analysis mode: {}", mode}
    }

    for (id, membr_name) in roots {
        if membr_name == look_for {
            let def = analysis.get_def(id)?;
            println!("Root: {:?} {:?} {:?} {}", id, def.kind, def.name, membr_name );
            traverse(id, def , &analysis, 0, &mut func_list)?;
        }
    }
    Ok(func_list)

}

fn traverse(id: rls_analysis::Id, defin: rls_analysis::Def , analysis: &AnalysisHost, mut indent: u32, funcs: &mut Vec<rls_analysis::Def>) 
    -> Result < () , Box<std::error::Error>> {
    println!("{}{:?} {:?} {:?}", " ".repeat(indent as usize), id, defin.kind, defin.name);
    match defin.kind {
        DefKind::Function 
        | DefKind::Method => {
            emit_sig(&analysis, &defin, &indent)?;
            funcs.push(defin.clone());
        }
        _ => (),
    }
    indent += 2;
    let mut children = analysis.for_each_child_def(id, |id, def| (id, def.clone()) )?;
    children.sort_unstable_by(|(_, def1), (_, def2)| def1.name.cmp(&def2.name));
    for (child, def) in children {
        traverse(child, def,  analysis, indent, funcs)?;
    }
    Ok(())
}

fn emit_sig (analysis: &AnalysisHost, defin: &rls_analysis::Def, indent: &u32) -> Result < (), Box<std::error::Error>>{
    let def = defin.clone();
    println!("{}Qualname: {} ", " ".repeat(*indent as usize +2), def.qualname );
    match def.sig {
        Some(x) => {
            println!("{}Signature: {}", " ".repeat(*indent as usize+2), x.text); //defin.value has the text as well
            for sig_el in x.defs{
                let qname = analysis.get_def(sig_el.id)?.qualname;
                println!("{}defs: id: {}, qualname: {}", " ".repeat(*indent as usize+4), sig_el.id, qname);
            }
            for sig_el in x.refs{
                let qname = analysis.get_def(sig_el.id)?.qualname;
                println!("{}refs: id: {}, qualname: {}", " ".repeat(*indent as usize+4), sig_el.id, qname);
            }
               // println!("{:?}", defin.sig);
        }
        None => println!("{}Signature (value): {}", " ".repeat(*indent as usize+2), def.value),
    }
    Ok(())
}
fn generate_analysis_files(dir : &path::Path) -> Result <(), Box<std::error::Error> >{
    let mut command = Command::new("cargo");

    let target_dir = dir.join("target").join("rls");
    let manifest_path = dir.join("Cargo.toml");

    let analysis_config = AnalysisConfig {
        //full_docs: true,
       // pub_only: true,
        signatures: true,
        ..Default::default()
    };

    command
        .arg("check")
        .arg("--manifest-path")
        .arg(manifest_path)
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", target_dir)
        //RUST_SAVE_ANALYSIS_CONFIG=' "reachable_only": true, "full_docs": true, "pub_only": false, 
            //"distro_crate": false, "signatures": false, "borrow_data": false'
        .env("RUST_SAVE_ANALYSIS_CONFIG", serde_json::to_string(&analysis_config)?,)
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    
    //command.current_dir(dir);
   /*  match target.kind {
        TargetKind::Library => {
            command.arg("--lib");
        }
        TargetKind::Binary => {
            command.args(&["--bin", &target.name]);
        }
    } */
    //command.args(&["rustc", "--lib", "--", "-Z", "save-analysis"]);
    command.arg("--lib");
    println!("Generating rls analysis data ...");
    println!("{:?}", command );
    let mut child = command.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        println!("ERROR!" );
        println!("{:?}", command );        
        panic!("child process spawned: {:?}", status);
    }
    Ok(())

}