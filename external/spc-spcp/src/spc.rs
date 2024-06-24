use std::io::{Read, Result, Error, ErrorKind, Seek, SeekFrom, BufReader};
use std::path::Path;
use std::fs::File;
use super::binary_reader::{BinaryRead, BinaryReader};
use crate::extended_id666::{ExtendedId666Chunk, ExtendedId666Data};
pub use super::id666::{Emulator, Id666Tag};

macro_rules! fail {
    ($expr:expr) => {
        return Err(Error::new(ErrorKind::Other, $expr))
    }
}

pub(crate) use fail;
use crate::metadata::Metadata;

pub const RAM_LEN: usize = 0x10000;
pub const REG_LEN: usize = 128;
pub const IPL_ROM_LEN: usize = 64;

const HEADER_LEN: usize = 33;
const HEADER_BYTES: &'static [u8; HEADER_LEN] =
    b"SNES-SPC700 Sound File Data v0.30";

#[derive(Clone)]
pub struct Spc {
    pub version_minor: u8,
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub psw: u8,
    pub sp: u8,
    pub id666_tag: Option<Id666Tag>,
    pub extended_id666: Option<ExtendedId666Data>,
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
        r.read_exact(&mut header)?;
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
        r.read_exact(&mut ram)?;
        let mut regs = [0; REG_LEN];
        r.read_exact(&mut regs)?;
        r.seek(SeekFrom::Start(0x101c0))?;
        let mut ipl_rom = [0; IPL_ROM_LEN];
        r.read_exact(&mut ipl_rom)?;
        let extended_id666 = ExtendedId666Data::load(&mut r).unwrap_or_else(|_| None);

        Ok(Spc {
            version_minor,
            pc,
            a,
            x,
            y,
            psw,
            sp,
            id666_tag,
            extended_id666,
            ram,
            regs,
            ipl_rom
        })
    }

    pub fn metadata(&self) -> Metadata {
        Metadata::new(self.id666_tag.as_ref(), self.extended_id666.as_ref())
    }
}

