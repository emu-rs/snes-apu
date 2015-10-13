use std::char;
use std::io::{Result, Error, ErrorKind, Seek, SeekFrom, BufReader};
use std::path::Path;
use std::fs::File;
use super::apu::{RAM_LEN, IPL_ROM_LEN};
use super::binary_reader::{ReadAll, BinaryRead, BinaryReader};

macro_rules! fail {
    ($expr:expr) => (return Err(Error::new(ErrorKind::Other, $expr)))
}

const REG_LEN: usize = 128;
const HEADER_LEN: usize = 33;
const HEADER_BYTES: &'static [u8; HEADER_LEN] =
    b"SNES-SPC700 Sound File Data v0.30";

pub struct Spc {
    pub version_minor: u8,
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub psw: u8,
    pub sp: u8,
    pub id666_tag: Option<Id666Tag>,
    pub ram: [u8; RAM_LEN],
    pub regs: [u8; REG_LEN],
    pub ipl_rom: [u8; IPL_ROM_LEN]
}

impl Spc {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Spc> {
        let file = try!(File::open(path));
        let mut r = BinaryReader::new(BufReader::new(file));

        let mut header = [0; HEADER_LEN];
        try!(r.read_all(&mut header));
        if header.iter().zip(HEADER_BYTES.iter()).any(|(x, y)| x != y) {
            fail!("Invalid header string");
        }

        if try!(r.read_le_u16()) != 0x1a1a {
            fail!("Invalid padding bytes");
        }

        let has_id666_tag = match try!(r.read_u8()) {
            0x1a => true,
            0x1b => false,
            _ => fail!("Unable to determine if file contains ID666 tag")
        };

        let version_minor = try!(r.read_u8());

        let pc = try!(r.read_le_u16());
        let a = try!(r.read_u8());
        let x = try!(r.read_u8());
        let y = try!(r.read_u8());
        let psw = try!(r.read_u8());
        let sp = try!(r.read_u8());

        let id666_tag = match has_id666_tag {
            true => {
                try!(r.seek(SeekFrom::Start(0x2e)));
                match Id666Tag::load(&mut r) {
                    Ok(x) => Some(x),
                    Err(e) => fail!(format!("Invalid ID666 tag: {}", e))
                }
            },
            false => None
        };

        try!(r.seek(SeekFrom::Start(0x100)));
        let mut ram = [0; RAM_LEN];
        try!(r.read_all(&mut ram));
        let mut regs = [0; REG_LEN];
        try!(r.read_all(&mut regs));
        try!(r.seek(SeekFrom::Start(0x101c0)));
        let mut ipl_rom = [0; IPL_ROM_LEN];
        try!(r.read_all(&mut ipl_rom));

        Ok(Spc {
            version_minor: version_minor,
            pc: pc,
            a: a,
            x: x,
            y: y,
            psw: psw,
            sp: sp,
            id666_tag: id666_tag,
            ram: ram,
            regs: regs,
            ipl_rom: ipl_rom
        })
    }
}

pub struct Id666Tag {
    pub song_title: String,
    pub game_title: String,
    pub dumper_name: String,
    pub comments: String,
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
}

impl Id666Tag {
    fn load<R: BinaryRead + Seek>(r: &mut R) -> Result<Id666Tag> {
        let song_title = try!(Id666Tag::read_string(r, 32));
        let game_title = try!(Id666Tag::read_string(r, 32));
        let dumper_name = try!(Id666Tag::read_string(r, 16));
        let comments = try!(Id666Tag::read_string(r, 32));

        // So, apparently, there's really no reliable way to detect whether or not
        //  an id666 tag is in text or binary format. I tried using the date field,
        //  but that's actually invalid in a lot of files anyways. I've read that
        //  the dumping emu can give clues (znes seems to dump binary files and
        //  snes9x seems to dump text), but these don't cover cases where the
        //  dumping emu is "unknown", so that sucks too. I've even seen some source
        //  where people try to differentiate based on the value of the psw register
        //  (lol). Ultimately, the most sensible solution I was able to dig up that
        //  seems to work on all of the .spc's I've tried is to just check if there
        //  appears to be textual data where the length and/or date fields should be.
        //  Still pretty icky, but it works pretty well.
        try!(r.seek(SeekFrom::Start(0x9e)));
        let is_text_format = match try!(Id666Tag::is_text_region(r, 11)) {
            true => {
                try!(r.seek(SeekFrom::Start(0xa9)));
                try!(Id666Tag::is_text_region(r, 3))
            },
            _ => false
        };

        try!(r.seek(SeekFrom::Start(0x9e)));

        let (date_dumped, seconds_to_play_before_fading_out, fade_out_length) =
            if is_text_format {
                let date_dumped = try!(Id666Tag::read_string(r, 11));
                let seconds_to_play_before_fading_out = try!(Id666Tag::read_number(r, 3));
                let fade_out_length = try!(Id666Tag::read_number(r, 5));

                (date_dumped, seconds_to_play_before_fading_out, fade_out_length)
            } else {
                // TODO: Find SPC's to test this with
                unimplemented!();

                /*let year = try!(r.read_le_u16());
                let month = try!(r.read_u8());
                let day = try!(r.read_u8());
                let date_dumped = format!("{}/{}/{}", month, day, year);

                try!(r.seek(SeekFrom::Start(0xa9)));
                let seconds_to_play_before_fading_out = try!(r.read_le_u16());
                try!(r.read_u8());
                let fade_out_length = try!(r.read_le_i32());

                (date_dumped, seconds_to_play_before_fading_out, fade_out_length)*/
            };

        let artist_name = try!(Id666Tag::read_string(r, 32));

        let default_channel_disables = try!(r.read_u8());

        let dumping_emulator = match try!(Id666Tag::read_digit(r)) {
            1 => Emulator::ZSnes,
            2 => Emulator::Snes9x,
            _ => Emulator::Unknown
        };

        Ok(Id666Tag {
            song_title: song_title,
            game_title: game_title,
            dumper_name: dumper_name,
            comments: comments,
            date_dumped: date_dumped,
            seconds_to_play_before_fading_out: seconds_to_play_before_fading_out,
            fade_out_length: fade_out_length,
            artist_name: artist_name,
            default_channel_disables: default_channel_disables,
            dumping_emulator: dumping_emulator
        })
    }

    fn read_string<R: BinaryRead>(r: &mut R, max_len: i32) -> Result<String> {
        // TODO: Reimplement as iterator or something similar
        let mut ret = "".to_string();
        let mut has_ended = false;
        for _ in 0..max_len {
            let b = try!(r.read_u8());
            if !has_ended {
                match char::from_u32(b as u32) {
                    Some(c) if b != 0 => ret.push(c),
                    _ => has_ended = true
                }
            }
        }
        Ok(ret)
    }

    fn is_text_region<R: BinaryRead>(r: &mut R, len: i32) -> Result<bool> {
        // TODO: This code is probably shit
        for _ in 0..len {
            let b = try!(r.read_u8());
            if b != 0 {
                if let Some(c) = char::from_u32(b as u32) {
                    if !c.is_digit(10) && c != '/' {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    fn read_digit<R: BinaryRead>(r: &mut R) -> Result<i32> {
        let d = try!(r.read_u8());
        Id666Tag::digit(d)
    }

    fn digit(d: u8) -> Result<i32> {
        match char::from_u32(d as u32) {
            Some(c) if c.is_digit(10) => Ok(c.to_digit(10).unwrap() as i32),
            _ => fail!("Expected numeric value")
        }
    }

    fn read_number<R: BinaryRead>(r: &mut R, max_len: i32) -> Result<i32> {
        let mut ret = 0;
        for _ in 0..max_len {
        let d = try!(r.read_u8());
            if d == 0 {
                break;
            }
            ret *= 10;
            ret += try!(Id666Tag::digit(d));
        }
        Ok(ret)
    }
}
