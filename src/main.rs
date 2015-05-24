use std::iter;
use std::env;

fn main() {
    match get_file_names() {
        Ok(file_names) =>
            for file_name in file_names {
                println!("{}", file_name);
            },
        
        Err(message) => {
            println!("ERROR: {}", message);
            std::process::exit(1);
        }
    }
}

fn get_file_names() -> Result<iter::Skip<env::Args>, String> {
    let args = env::args();
    match args.len() {
        1 => Err("No file(s) specified".to_string()),
        _ => Ok(args.skip(1))
    }
}
