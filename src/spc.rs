use binary_reader::BinaryReader;

use std::io::{Result, Error, ErrorKind, Read, BufReader};
use std::fs::File;

type SpcResult = Result<Spc>;

struct Spc {
    header: String,
    version_minor: i32,
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    psw: u8,
    sp: u8,
    id666_tag: Option<Id666>,
    // TODO: ram, register_data, ipl_rom
}

impl Spc {
    pub fn load(file_name: String) -> SpcResult {
        let mut file = try!(File::open(file_name));
        let mut r = BinaryReader::new(BufReader::new(file));

        let mut headerBuf: [u8; 33] = [0; 33];
        try!(r.read(&mut headerBuf));
        
        unrecognized_header("dagnabbit")
    }
}

fn unrecognized_header(message: &str) -> SpcResult {
    Err(Error::new(ErrorKind::Other, message))
}

struct Id666 {
    song_title: String,
    game_title: String,
    dumper_name: String,
    date_dumped: String,
    seconds_to_play_before_fading_out: i32,
    fade_out_length: i32,
    artist_name: String,
    default_channel_disables: u8,
    dumping_emulator: Emulator
}

enum Emulator {
    Unknown,
    ZSnes,
    Snes9x
}
