use std::char;
use std::io::{Read, Result, Error, ErrorKind, Seek, SeekFrom, BufReader};
use std::path::Path;
use std::fs::File;
use std::time::Duration;
use super::binary_reader::{ReadAll, BinaryRead, BinaryReader};
use super::string_decoder::decode_string;

macro_rules! fail {
    ($expr:expr) => {
        return Err(Error::new(ErrorKind::Other, $expr))
    }
}

pub const RAM_LEN: usize = 0x10000;
pub const REG_LEN: usize = 128;
pub const IPL_ROM_LEN: usize = 64;

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
        let file = File::open(path)?;
        Spc::from_reader(BufReader::new(file))
    }

    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Spc> {
        let mut r = BinaryReader::new(reader);

        let mut header = [0; HEADER_LEN];
        r.read_all(&mut header)?;
        if header.iter().zip(HEADER_BYTES.iter()).any(|(x, y)| x != y) {
            fail!("Invalid header string");
        }

        if r.read_le_u16()? != 0x1a1a {
            fail!("Invalid padding bytes");
        }

        let has_id666_tag = match r.read_u8()? {
            0x00 | 0x1a => true,  // fix for older Super MIDI Pak SPCs
            0x1b => false,
            _ => fail!("Unable to determine if file contains ID666 tag")
        };

        let version_minor = r.read_u8()?;

        let pc = r.read_le_u16()?;
        let a = r.read_u8()?;
        let x = r.read_u8()?;
        let y = r.read_u8()?;
        let psw = r.read_u8()?;
        let sp = r.read_u8()?;

        let id666_tag = match has_id666_tag {
            true => {
                r.seek(SeekFrom::Start(0x2e))?;
                match Id666Tag::load(&mut r) {
                    Ok(x) => Some(x),
                    Err(e) => fail!(format!("Invalid ID666 tag: {}", e))
                }
            },
            false => None
        };

        r.seek(SeekFrom::Start(0x100))?;
        let mut ram = [0; RAM_LEN];
        r.read_all(&mut ram)?;
        let mut regs = [0; REG_LEN];
        r.read_all(&mut regs)?;
        r.seek(SeekFrom::Start(0x101c0))?;
        let mut ipl_rom = [0; IPL_ROM_LEN];
        r.read_all(&mut ipl_rom)?;
        let mut extended_id666_data: Vec<u8> = Vec::new();
        r.read_to_end(&mut extended_id666_data)?;

        Ok(Spc {
            version_minor,
            pc,
            a,
            x,
            y,
            psw,
            sp,
            id666_tag,
            ram,
            regs,
            ipl_rom
        })
    }
}

const DEFAULT_PLAY_TIME_SEC: i32 = 120;
const DEFAULT_FADEOUT_TIME_MS: i32 = 10000;

pub struct Id666Tag {
    pub song_title: String,
    pub game_title: String,
    pub dumper_name: String,
    pub comments: String,
    pub date_dumped: String,
    pub play_time: Duration,
    pub fadeout_time: Duration,
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
        let song_title = Id666Tag::read_string(r, 32)?;
        let game_title = Id666Tag::read_string(r, 32)?;
        let dumper_name = Id666Tag::read_string(r, 16)?;
        let comments = Id666Tag::read_string(r, 32)?;

        // So, apparently, there's really no reliable way to detect whether or not
        //  an id666 tag is in text or binary format. I tried using the date field,
        //  but that's actually invalid in a lot of files anyways. I've read that
        //  the dumping emu can give clues (zsnes seems to dump binary files and
        //  snes9x seems to dump text), but these don't cover cases where the
        //  dumping emu is "unknown", so that sucks too. I've even seen some source
        //  where people try to differentiate based on the value of the psw register
        //  (lol). Ultimately, the most sensible solution I was able to dig up that
        //  seems to work on all of the .spc's I've tried is to just check if there
        //  appears to be textual data where the length and/or date fields should be.
        //  Still pretty icky, but it works pretty well.
        r.seek(SeekFrom::Start(0x9e))?;
        let is_text_format = match Id666Tag::is_text_region(r, 11)? {
            true => {
                r.seek(SeekFrom::Start(0xa9))?;
                Id666Tag::is_text_region(r, 3)?
            },
            _ => false
        };

        r.seek(SeekFrom::Start(0x9e))?;

        let (date_dumped, play_time_sec, fadeout_ms) =
            if is_text_format {
                let date_dumped = Id666Tag::read_string(r, 11)?;
                let play_time_sec = Id666Tag::read_number(r, 3, DEFAULT_PLAY_TIME_SEC)?;
                let fadeout_ms = Id666Tag::read_number(r, 5, DEFAULT_FADEOUT_TIME_MS)?;

                (date_dumped, play_time_sec, fadeout_ms)
            } else {
                let year = r.read_le_u16()?;
                let month = r.read_u8()?;
                let day = r.read_u8()?;
                let date_dumped = format!("{}/{}/{}", month, day, year);

                r.seek(SeekFrom::Start(0xa9))?;
                let play_time_sec = Id666Tag::read_number(r, 3, DEFAULT_PLAY_TIME_SEC)?;
                let fadeout_ms = Id666Tag::read_number(r, 4, DEFAULT_FADEOUT_TIME_MS)?;

                (date_dumped, play_time_sec, fadeout_ms)
            };

        let artist_name = Id666Tag::read_string(r, 32)?;

        let default_channel_disables = r.read_u8()?;

        let dumping_emulator = match Id666Tag::read_number(r, 1, 0)? {
            1 => Emulator::ZSnes,
            2 => Emulator::Snes9x,
            _ => Emulator::Unknown
        };

        Ok(Id666Tag {
            song_title,
            game_title,
            dumper_name,
            comments,
            date_dumped,
            play_time: Duration::from_secs(play_time_sec as u64),
            fadeout_time: Duration::from_millis(fadeout_ms as u64),
            artist_name,
            default_channel_disables,
            dumping_emulator
        })
    }

    fn read_string<R: BinaryRead>(r: &mut R, max_len: i32) -> Result<String> {
        let string_bytes = (0..max_len)
            .map(|_| r.read_u8())
            .collect::<Result<Vec<u8>>>()?;

        let end = string_bytes
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(max_len as usize);

        decode_string(&string_bytes[..end])
    }

    fn is_text_region<R: BinaryRead>(r: &mut R, len: i32) -> Result<bool> {
        let region_bytes = (0..len)
            .map(|_| r.read_u8())
            .collect::<Result<Vec<u8>>>()?;

        Ok(
            region_bytes
                .into_iter()
                .filter_map(|b| {
                    if b == 0 {
                        return None;
                    }
                    char::from_u32(b as u32)
                })
                .all(|c| c.is_digit(10) || c == '/')
        )
    }

    fn read_number<R: BinaryRead>(r: &mut R, max_len: i32, default: i32) -> Result<i32> {
        let num_string = Id666Tag::read_string(r, max_len)?;

        match i32::from_str_radix(&num_string, 10) {
            Ok(0) => Ok(default),
            Ok(result) => Ok(result),
            Err(e) => fail!(e)
        }
    }
}
