mod binary_reader;
mod spc;
mod snes_apu;

use std::iter;
use std::env;
use std::io::{Result, Error, ErrorKind};

use spc::{Spc, Emulator};

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
    println!(" Version Minor: {}", spc.version_minor);
    println!(" PC: {}", spc.pc);
    println!(" A: {}", spc.a);
    println!(" X: {}", spc.x);
    println!(" Y: {}", spc.y);
    println!(" PSW: {}", spc.psw);
    println!(" SP: {}", spc.sp);

    if let Some(id666_tag) = spc.id666_tag {
        println!(" ID666 tag present:");
        println!("  Song title: {}", id666_tag.song_title);
        println!("  Game title: {}", id666_tag.game_title);
        println!("  Dumper name: {}", id666_tag.dumper_name);
        println!("  Comments: {}", id666_tag.comments);
        println!("  Date dumped (MM/DD/YYYY): {}", id666_tag.date_dumped);
        println!("  Seconds to play before fading out: {}", id666_tag.seconds_to_play_before_fading_out);
        println!("  Fade out length: {}ms", id666_tag.fade_out_length);
        println!("  Artist name: {}", id666_tag.artist_name);
        println!("  Default channel disables: {}", id666_tag.default_channel_disables);
        println!("  Dumping emulator: {}", match id666_tag.dumping_emulator {
            Emulator::Unknown => "Unknown",
            Emulator::ZSnes => "ZSnes",
            Emulator::Snes9x => "Snes9x"
        });
    } else {
        println!(" No ID666 tag present.");
    }

    Ok(())
}
