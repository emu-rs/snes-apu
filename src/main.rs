mod binary_reader;
mod spc;

use std::iter;
use std::env;
use std::io::{Result, Error, ErrorKind};

use spc::Spc;

fn main() {
    if let Err(e) = play_spc_files() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn play_spc_files() -> Result<()> {
    for file_name in try!(get_file_names()) {
        try!(play_spc_file(&file_name));
    }
    Ok(())
}

fn get_file_names() -> Result<iter::Skip<env::Args>> {
    let args = env::args();
    match args.len() {
        1 => Err(Error::new(ErrorKind::Other, "No file(s) specified")),
        _ => Ok(args.skip(1))
    }
}

fn play_spc_file(file_name: &String) -> Result<()> {
    let spc = try!(Spc::load(file_name));

    println!("SPC: {}", file_name);
    println!(" Header: {}", String::from_utf8(spc.header.iter().cloned().collect()).unwrap());

    println!(" Version Minor: {}", spc.version_minor);
    println!(" PC: {}", spc.pc);
    println!(" A: {}", spc.a);
    println!(" X: {}", spc.x);
    println!(" Y: {}", spc.y);
    println!(" PSW: {}", spc.psw);
    println!(" SP: {}", spc.sp);

    // TODO: ID666 Tag

    

    unimplemented!();
}
