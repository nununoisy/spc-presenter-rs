use crate::dsp::dsp::Dsp;
use crate::dsp::dsp_helpers;
use crate::dsp::stereo::StereoChannel;

const FIR_TAPS: usize = 8;

impl Dsp {
    fn calculate_fir(&self, channel: StereoChannel, index: usize) -> i32 {
        let sample = self.echo_history.get(channel)[(self.echo_history_offset + index + 1) % FIR_TAPS] as i32;
        (sample * (self.fir[index] as i8) as i32) >> 6
    }

    pub(super) fn echo_output(&self, channel: StereoChannel) -> i32 {
        let master = dsp_helpers::multiply_volume(self.master_output.into_inner(channel), self.master_volume.into_inner(channel));
        let echo = dsp_helpers::multiply_volume(self.echo_input.into_inner(channel), self.echo_volume.into_inner(channel));
        dsp_helpers::clamp(master + echo)
    }

    fn echo_read(&mut self, channel: StereoChannel) {
        let address = self.echo_address as u32 + 2 * channel.as_offset() as u32;
        let lo = self.emulator().read_u8(address) as u16;
        let hi = self.emulator().read_u8(address + 1) as u16;
        let sample = ((hi << 8) | lo) as i16;
        self.echo_history.get_mut(channel)[self.echo_history_offset] = sample >> 1;
    }

    fn echo_write(&mut self, channel: StereoChannel) {
        if self.echo_write_enabled {
            let address = self.echo_address as u32 + 2 * channel.as_offset() as u32;
            let sample = self.echo_output.into_inner(channel);
            let lo = (sample & 0xFF) as u8;
            let hi = ((sample >> 8) & 0xFF) as u8;
            self.emulator().write_u8(address, lo);
            self.emulator().write_u8(address + 1, hi);
        }
        self.echo_output.set(channel, 0);
    }

    pub(super) fn echo22(&mut self) {
        self.echo_history_offset = (self.echo_history_offset + 1) % FIR_TAPS;

        self.echo_address = self.echo_start_address.wrapping_add(self.echo_pos as u16);
        self.echo_read(StereoChannel::Left);

        let l = self.calculate_fir(StereoChannel::Left, 0);
        let r = self.calculate_fir(StereoChannel::Right, 0);

        self.echo_input.set_left(dsp_helpers::cast_arb_int(l, 17));
        self.echo_input.set_right(dsp_helpers::cast_arb_int(r, 17));
    }

    pub(super) fn echo23(&mut self) {
        let l = self.calculate_fir(StereoChannel::Left, 1) + self.calculate_fir(StereoChannel::Left, 2);
        let r = self.calculate_fir(StereoChannel::Right, 1) + self.calculate_fir(StereoChannel::Right, 2);

        self.echo_input.set_left(dsp_helpers::cast_arb_int(self.echo_input.into_inner_left() + l, 17));
        self.echo_input.set_right(dsp_helpers::cast_arb_int(self.echo_input.into_inner_right() + r, 17));

        self.echo_read(StereoChannel::Right);
    }

    pub(super) fn echo24(&mut self) {
        let l = self.calculate_fir(StereoChannel::Left, 3) + self.calculate_fir(StereoChannel::Left, 4) + self.calculate_fir(StereoChannel::Left, 5);
        let r = self.calculate_fir(StereoChannel::Right, 3) + self.calculate_fir(StereoChannel::Right, 4) + self.calculate_fir(StereoChannel::Right, 5);

        self.echo_input.set_left(dsp_helpers::cast_arb_int(self.echo_input.into_inner_left() + l, 17));
        self.echo_input.set_right(dsp_helpers::cast_arb_int(self.echo_input.into_inner_right() + r, 17));
    }

    pub(super) fn echo25(&mut self) {
        let mut l = self.echo_input.into_inner_left() + self.calculate_fir(StereoChannel::Left, 6);
        let mut r = self.echo_input.into_inner_right() + self.calculate_fir(StereoChannel::Right, 6);

        l = dsp_helpers::cast_arb_int(l, 16);
        r = dsp_helpers::cast_arb_int(r, 16);

        l += dsp_helpers::cast_arb_int(self.calculate_fir(StereoChannel::Left, 7), 16);
        r += dsp_helpers::cast_arb_int(self.calculate_fir(StereoChannel::Right, 7), 16);

        self.echo_input.set_left(dsp_helpers::cast_arb_int(dsp_helpers::clamp(l) & !1, 17));
        self.echo_input.set_right(dsp_helpers::cast_arb_int(dsp_helpers::clamp(r) & !1, 17));
    }

    pub(super) fn echo26(&mut self) {
        self.master_output.set_left(self.echo_output(StereoChannel::Left));

        let echo_feedback = (self.echo_feedback as i8) as i32;
        let l = self.echo_output.into_inner_left() + dsp_helpers::cast_arb_int((self.echo_input.into_inner_left() * echo_feedback) >> 7, 16);
        let r = self.echo_output.into_inner_right() + dsp_helpers::cast_arb_int((self.echo_input.into_inner_right() * echo_feedback) >> 7, 16);

        self.echo_output.set_left(dsp_helpers::cast_arb_int(dsp_helpers::clamp(l) & !1, 17));
        self.echo_output.set_right(dsp_helpers::cast_arb_int(dsp_helpers::clamp(r) & !1, 17));
    }

    pub(super) fn echo27(&mut self) {
        let out_l = self.master_output.into_inner_left();
        let out_r = self.echo_output(StereoChannel::Right);
        self.master_output.set_left(0);
        self.master_output.set_right(0);

        if !self.master_mute {
            self.output_buffer.write_sample(out_l as i16, out_r as i16);
        } else {
            self.output_buffer.write_sample(0, 0);
        }
    }

    pub(super) fn echo28(&mut self) {
        self.echo_write_enabled = self.l_echo_write_enabled;
    }

    pub(super) fn echo29(&mut self) {
        self.echo_start_address = self.l_echo_start_address;
        self.echo_length = self.calculate_echo_length();

        self.echo_pos += 4;
        if self.echo_pos >= self.echo_length {
            self.echo_pos = 0;
        }

        self.echo_write(StereoChannel::Left);

        self.echo_write_enabled = self.l_echo_write_enabled;
    }

    pub(super) fn echo30(&mut self) {
        self.echo_write(StereoChannel::Right);
    }
}
