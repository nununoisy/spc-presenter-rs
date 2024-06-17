use std::convert::TryInto;
use std::io::{Read, Result, Error, ErrorKind, Seek, SeekFrom, BufReader};
use std::ops::Deref;
use std::time::Duration;
use crate::spc::Id666Tag;
use super::binary_reader::{BinaryRead, BinaryReader};
use super::id666::Emulator;

#[derive(Clone, Debug)]
pub enum ExtendedId666Chunk {
    // Original Id666 tag data
    SongTitle(String),
    GameTitle(String),
    ArtistName(String),
    DumperName(String),
    DateDumped(String),
    DumpingEmulator(Emulator),
    Comments(String),
    // Extended items
    OstTitle(String),
    OstDisc(u16),
    OstTrack((u8, Option<char>)),
    PublisherName(String),
    CopyrightYear(u16),
    // Playback items
    IntroductionLength(Duration),
    LoopLength(Duration),
    EndLength(Duration),
    FadeoutLength(Duration),
    MutedVoices([bool; 8]),
    PreferredLoopCount(u16),
    PreampLevel(u16)
}

impl ExtendedId666Chunk {
    fn read<R: BinaryRead + Seek>(r: &mut R) -> Result<Option<Self>> {
        Ok(Some(match r.read_u8() {
            Ok(id) => match id {
                // Padding, try with next byte
                0x00 => return Self::read(r),

                0x01 => Self::SongTitle(Self::read_string_item(r)?),
                0x02 => Self::GameTitle(Self::read_string_item(r)?),
                0x03 => Self::ArtistName(Self::read_string_item(r)?),
                0x04 => Self::DumperName(Self::read_string_item(r)?),
                0x05 => {
                    let raw_date = Self::read_integer_item(r)?;
                    let year = raw_date >> 16;
                    let month = (raw_date >> 8) & 0xFF;
                    let day = raw_date & 0xFF;
                    Self::DateDumped(format!("{}/{}/{}", month, day, year))
                },
                0x06 => match Self::read_data_item(r)? {
                    1 => Self::DumpingEmulator(Emulator::ZSnes),
                    2 => Self::DumpingEmulator(Emulator::Snes9x),
                    _ => Self::DumpingEmulator(Emulator::Unknown)
                },
                0x07 => Self::Comments(Self::read_string_item(r)?),

                0x10 => Self::OstTitle(Self::read_string_item(r)?),
                0x11 => Self::OstDisc(Self::read_data_item(r)?),
                0x12 => {
                    let raw_track = Self::read_data_item(r)?;
                    let track = (raw_track >> 8) as u8;
                    let track_char = match (raw_track & 0xFF) as u8 {
                        0 => None,
                        c => Some(c as char)
                    };
                    Self::OstTrack((track, track_char))
                },
                0x13 => Self::PublisherName(Self::read_string_item(r)?),
                0x14 => Self::CopyrightYear(Self::read_data_item(r)?),

                0x30 => Self::IntroductionLength(Self::read_duration_integer(r)?),
                0x31 => Self::LoopLength(Self::read_duration_integer(r)?),
                0x32 => Self::EndLength(Self::read_duration_integer(r)?),
                0x33 => Self::FadeoutLength(Self::read_duration_integer(r)?),
                0x34 => {
                    let raw_muted = Self::read_data_item(r)?;
                    let muted: [bool; 8] = (0..8)
                        .map(|i| ((raw_muted >> (7 - i)) & 1) == 1)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap();
                    Self::MutedVoices(muted)
                },
                0x35 => Self::PreferredLoopCount(Self::read_data_item(r)?),
                0x36 => Self::PreampLevel(Self::read_data_item(r)?),

                _ => return Err(Error::new(ErrorKind::InvalidData, "Unknown extended ID"))
            },
            Err(e) => {
                return if e.kind() == ErrorKind::UnexpectedEof {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }))
    }

    fn read_data_item<R: BinaryRead + Seek>(r: &mut R) -> Result<u16> {
        if r.read_u8()? != 0 {
            return Err(Error::new(ErrorKind::InvalidData, "Expected data item"));
        }

        r.read_le_u16()
    }

    fn read_integer_item<R: BinaryRead + Seek>(r: &mut R) -> Result<u32> {
        if r.read_u8()? != 4 {
            return Err(Error::new(ErrorKind::InvalidData, "Expected integer item"));
        }
        if r.read_le_u16()? != 4 {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid integer item length"));
        }

        r.read_le_u32()
    }

    fn read_duration_integer<R: BinaryRead + Seek>(r: &mut R) -> Result<Duration> {
        let raw_duration = Self::read_integer_item(r)?;
        Ok(Duration::from_secs_f64(raw_duration as f64 / 64000.0))
    }

    fn read_string_item<R: BinaryRead + Seek>(r: &mut R) -> Result<String> {
        if r.read_u8()? != 1 {
            return Err(Error::new(ErrorKind::InvalidData, "Expected string item"));
        }
        let len = r.read_le_u16()? as i32;
        let max_len = len + ((4 - (len % 4)) % 4);
        r.read_string(max_len)
    }
}

#[derive(Clone)]
pub struct ExtendedId666Data {
    chunks: Vec<ExtendedId666Chunk>
}

impl ExtendedId666Data {
    pub(super) fn load<R: BinaryRead + Seek>(r: &mut R) -> Result<Option<Self>> {
        let mut magic = [0u8; 4];
        let mut size = [0u8; 4];
        let bytes_read = r.read(&mut magic)? + r.read(&mut size)?;
        if bytes_read != 8 {
            return Ok(None);
        }

        if &magic != b"xid6" {
            // Ignore malformed header
            return Ok(None);
        }
        let size = u32::from_le_bytes(size) as usize;
        if size == 0 {
            return Ok(None);
        }

        let mut chunks: Vec<ExtendedId666Chunk> = Vec::new();
        while let Some(chunk) = ExtendedId666Chunk::read(r)? {
            println!("{:?}", chunk);
            chunks.push(chunk);
        }

        Ok(Some(Self {
            chunks
        }))
    }

    pub fn augment_id666_tag(&self, tag: &mut Id666Tag) {
        for chunk in self.chunks.iter() {
            match chunk {
                ExtendedId666Chunk::SongTitle(s) => tag.song_title = s.clone(),
                ExtendedId666Chunk::ArtistName(s) => tag.artist_name = s.clone(),
                ExtendedId666Chunk::GameTitle(s) => tag.game_title = s.clone(),
                ExtendedId666Chunk::DumperName(s) => tag.dumper_name = s.clone(),
                ExtendedId666Chunk::DateDumped(s) => tag.date_dumped = s.clone(),
                ExtendedId666Chunk::DumpingEmulator(e) => tag.dumping_emulator = e.clone(),
                ExtendedId666Chunk::Comments(s) => tag.comments = s.clone(),

                // TODO duration

                _ => ()
            }
        }
    }
}

impl Deref for ExtendedId666Data {
    type Target = [ExtendedId666Chunk];

    fn deref(&self) -> &Self::Target {
        &self.chunks
    }
}

#[macro_export]
macro_rules! search_xid6 {
    ($data: expr, $tag: ident) => {{
        $data.iter().find_map(|chunk| match chunk {
            $crate::extended_id666::ExtendedId666Chunk::$tag(v) => Some(v.clone()),
            _ => None
        })
    }};
}

pub use search_xid6;
