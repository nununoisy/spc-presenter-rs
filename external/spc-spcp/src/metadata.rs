use std::time::Duration;
use crate::extended_id666::ExtendedId666Data;
use crate::id666::Emulator;
use crate::search_xid6;
use crate::spc::Id666Tag;

pub struct Metadata<'a> {
    id6: Option<&'a Id666Tag>,
    xid6: Option<&'a ExtendedId666Data>
}

macro_rules! standard_item_impl {
    ($name: ident, $xid6_chunk: ident, $ret: ty) => {
        pub fn $name(&self) -> Option<$ret> {
            if let Some(xid6) = self.xid6 {
                if let Some(value) = search_xid6!(xid6, $xid6_chunk) {
                    return Some(value);
                }
            }

            Some(self.id6?.$name.clone())
        }
    };
}

impl<'a> Metadata<'a> {
    pub fn new(id6: Option<&'a Id666Tag>, xid6: Option<&'a ExtendedId666Data>) -> Self {
        Self {
            id6,
            xid6
        }
    }

    standard_item_impl!(song_title, SongTitle, String);
    standard_item_impl!(artist_name, ArtistName, String);
    standard_item_impl!(game_title, GameTitle, String);
    standard_item_impl!(dumper_name, DumperName, String);
    standard_item_impl!(comments, Comments, String);
    standard_item_impl!(date_dumped, DateDumped, String);
    standard_item_impl!(dumping_emulator, DumpingEmulator, Emulator);
    standard_item_impl!(muted_voices, MutedVoices, [bool; 8]);

    pub fn play_time(&self, loop_count: Option<u32>) -> Option<(Duration, Duration)> {
        let mut play_time = self.xid6
            .and_then(|xid6|  search_xid6!(xid6, IntroductionLength))
            .or_else(|| self.id6.map(|id6| id6.play_time))?;
        let mut fadeout_time = self.xid6
            .and_then(|xid6|  search_xid6!(xid6, FadeoutLength))
            .or_else(|| self.id6.map(|id6| id6.fadeout_time))?;

        if let Some(xid6) = self.xid6 {
            if let Some(loop_time) = search_xid6!(xid6, LoopLength) {
                let loop_count = loop_count
                    .or_else(|| search_xid6!(xid6, PreferredLoopCount).map(|count| count as u32))
                    .unwrap_or(1);
                play_time += loop_count * loop_time;
            }

            if let Some(end_time) = search_xid6!(xid6, EndLength) {
                play_time += end_time;
                fadeout_time = Duration::ZERO;
            }
        }

        Some((play_time, fadeout_time))
    }

    pub fn ost_info(&self) -> Option<OstInfo> {
        let xid6 = self.xid6?;
        let title = search_xid6!(xid6, OstTitle)?;
        let disc = search_xid6!(xid6, OstDisc).unwrap_or(1);
        let track = search_xid6!(xid6, OstTrack)?;
        let publisher = search_xid6!(xid6, PublisherName)?;
        let copyright_year = search_xid6!(xid6, CopyrightYear)?;

        Some(OstInfo {
            title,
            disc,
            track,
            publisher,
            copyright_year
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct OstInfo {
    pub title: String,
    pub disc: u16,
    pub track: (u8, Option<char>),
    pub publisher: String,
    pub copyright_year: u16
}
