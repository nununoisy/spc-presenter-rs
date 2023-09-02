use std::iter::zip;
use ffmpeg_next::{software, format::{Sample, sample::Type}, ChannelLayout, frame, util::mathematics::Rescale, Rounding, Rational};
use crate::video_builder;
use crate::video_builder::VideoBuilderUnwrap;

pub struct Resampler {
    rsp_ctx: software::resampling::Context
}

impl Resampler {
    pub fn new(sample_rate: u32) -> Result<Self, String> {
        let rsp_ctx = software::resampler(
            (Sample::I16(Type::Packed), ChannelLayout::STEREO, 32000),
            (Sample::I16(Type::Packed), ChannelLayout::STEREO, sample_rate)
        ).vb_unwrap()?;

        Ok(Self {
            rsp_ctx
        })
    }

    pub fn run(&mut self, l_audio: &[i16], r_audio: &[i16]) -> Result<Vec<i16>, String> {
        if l_audio.len() != r_audio.len() {
            return Err("Audio buffer length mismatch!".to_string());
        }

        let mut in_frame = frame::Audio::new(Sample::I16(Type::Packed), l_audio.len(), ChannelLayout::STEREO);
        in_frame.set_rate(32000);

        let mut packed_samples: Vec<(i16, i16)> = zip(l_audio.iter(), r_audio.iter())
            .map(|(l, r)| (*l, *r))
            .collect();
        in_frame.plane_mut::<(i16, i16)>(0).copy_from_slice(&packed_samples);

        let out_samples = (l_audio.len() as i64).rescale_with( Rational::from(self.rsp_ctx.output().rate as f64), Rational::from(32000.0), Rounding::Up);
        let mut out_frame = frame::Audio::new(Sample::I16(Type::Packed), out_samples as usize, ChannelLayout::STEREO);
        self.rsp_ctx.run(&in_frame, &mut out_frame).vb_unwrap()?;

        let mut result: Vec<i16> = Vec::with_capacity(out_samples as usize * 2);
        for (l, r) in out_frame.plane::<(i16, i16)>(0) {
            result.push(*l);
            result.push(*r);
        }

        Ok(result)
    }
}
