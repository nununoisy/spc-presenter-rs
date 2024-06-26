use std::char;
use std::io::{Result, Error, ErrorKind, Seek, SeekFrom};
use std::time::Duration;
use super::spc::fail;
use super::binary_reader::BinaryRead;

const DEFAULT_PLAY_TIME_SEC: i32 = 120;
const DEFAULT_FADEOUT_TIME_MS: i32 = 10000;

#[derive(Clone)]
pub struct Id666Tag {
    pub song_title: String,
    pub game_title: String,
    pub dumper_name: String,
    pub comments: String,
    pub date_dumped: String,
    pub play_time: Duration,
    pub fadeout_time: Duration,
    pub artist_name: String,
    pub muted_voices: [bool; 8],
    pub dumping_emulator: Emulator
}

#[derive(Clone, Debug)]
pub enum Emulator {
    Unknown,
    ZSnes,
    Snes9x
}

impl Id666Tag {
    pub(super) fn load<R: BinaryRead + Seek>(r: &mut R) -> Result<Self> {
        let song_title = r.read_string(32)?;
        let game_title = r.read_string(32)?;
        let dumper_name = r.read_string(16)?;
        let comments = r.read_string(32)?;

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
                let date_dumped = r.read_string(11)?;
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

        let artist_name = r.read_string(32)?;

        let raw_muted = r.read_u8()?;
        let muted_voices: [bool; 8] = (0..8)
            .map(|i| ((raw_muted >> (7 - i)) & 1) == 1)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

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
            muted_voices,
            dumping_emulator
        })
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
        let num_string = r.read_string(max_len)?;

        if num_string.is_empty() {
            // Hack for some strangely-tagged SPCs
            return Ok(default);
        }

        match i32::from_str_radix(&num_string, 10) {
            Ok(0) => Ok(default),
            Ok(result) => Ok(result),
            Err(e) => fail!(e)
        }
    }
}
