use binary_reader::{ReadAll, BinaryRead, BinaryReader};

use std::io::{Result, Error, ErrorKind, Seek, SeekFrom, BufReader};
use std::fs::File;

pub type SpcResult = Result<Spc>;

pub struct Spc {
    pub header: [u8; 33],
    pub version_minor: u8,
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub psw: u8,
    pub sp: u8,
    //pub id666_tag: Option<Id666>,
    pub ram: [u8; 0x10000],
    pub regs: [u8; 128],
    pub ipl_rom: [u8; 64]
}

impl Spc {
    pub fn load(file_name: &String) -> SpcResult {
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

        // TODO: Read ID666 tag if available

        try!(r.seek(SeekFrom::Start(0x100)));
        let mut ram = [0; 0x10000];
        try!(r.read_all(&mut ram));
        let mut regs = [0; 128];
        try!(r.read_all(&mut regs));
        try!(r.seek(SeekFrom::Start(0x101c0)));
        let mut ipl_rom = [0; 64];
        try!(r.read_all(&mut ipl_rom));
        
        Ok(Spc {
            header: header,
            version_minor: version_minor,
            pc: pc,
            a: a,
            x: x,
            y: y,
            psw: psw,
            sp: sp,
            ram: ram,
            regs: regs,
            ipl_rom: ipl_rom
        })
    }
}

/*pub struct Id666 {
    pub song_title: String,
    pub game_title: String,
    pub dumper_name: String,
    pub date_dumped: String,
    pub seconds_to_play_before_fading_out: i32,
    pub fade_out_length: i32,
    pub artist_name: String,
    pub default_channel_disables: u8,
    pub dumping_emulator: Emulator
}

pub enum Emulator {
    Unknown,
    ZSnes,
    Snes9x
}*/
