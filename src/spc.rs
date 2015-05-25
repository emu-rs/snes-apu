use binary_reader::{ReadAll, BinaryRead, BinaryReader};

use std::io::{Result, Error, ErrorKind, BufReader};
use std::fs::File;

pub type SpcResult = Result<Spc>;

pub struct Spc {
    header: [u8; 33],
    version_minor: u8,
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    psw: u8,
    sp: u8,
    //id666_tag: Option<Id666>,
    // TODO: ram, register_data, ipl_rom
}

impl Spc {
    pub fn load(file_name: String) -> SpcResult {
        macro_rules! bad_header {
            ($add_info:expr) => ({
                let message_text = "Unrecognized SPC header".to_string();
                let message =
                    match $add_info {
                        "" => message_text,
                        _ => message_text + " (" + $add_info + ")"
                    };
                return Err(Error::new(ErrorKind::Other, message));
            });
            () => (bad_header!(""))
        }

        macro_rules! assert_header {    
            ($cond:expr, $message:expr) => (if !$cond { bad_header!($message); });
            ($cond:expr) => (assert_header!($cond, ""))
        }
        
        let file = try!(File::open(file_name));
        let mut r = BinaryReader::new(BufReader::new(file));

        macro_rules! u8 {
            () => (try!(r.read_u8()))
        }

        macro_rules! u16 {
            () => (try!(r.read_le_u16()))
        }

        macro_rules! i32 {
            () => (try!(r.read_le_i32()))
        }

        let mut header = [0; 33];
        try!(r.read_all(&mut header));
        assert_header!(
            header.iter()
                .zip(b"SNES-SPC700 Sound File Data v0.30".iter())
                .all(|(x, y)| x == y),
            "Invalid header string");

        assert_header!(u16!() == 0x1a1a);

        let has_id666_tag = match u8!() {
            0x1a => true,
            0x1b => false,
            _ => bad_header!("Unable to determine if file contains ID666 tag")
        };

        let version_minor = u8!();

        let pc = u16!();
        let a = u8!();
        let x = u8!();
        let y = u8!();
        let psw = u8!();
        let sp = u8!();
        
        Ok(Spc {
            header: header,
            version_minor: version_minor,
            pc: pc,
            a: a,
            x: x,
            y: y,
            psw: psw,
            sp: sp
        })
    }
}

/*pub struct Id666 {
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

pub enum Emulator {
    Unknown,
    ZSnes,
    Snes9x
}*/
