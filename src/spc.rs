use std::fs::File;

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
    fn load(file_name: String) -> Result<Spc> {
        let mut file = try!(File::open(file_name));
        
    }
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
