use std::cmp::Ordering;
use ringbuf::{HeapRb, Rb};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Rect, Transform};
use super::{C_0, Visualizer, APU_STATE_BUF_SIZE, ChannelState, ChannelSettings};

#[derive(Copy, Clone, PartialEq)]
enum PianoKey {
    WhiteLeft,
    WhiteCenter,
    WhiteRight,
    WhiteFull,
    Black
}

const PIANO_KEYS: [PianoKey; 12] = [
    PianoKey::WhiteLeft,    // C
    PianoKey::Black,        // C#
    PianoKey::WhiteCenter,  // D
    PianoKey::Black,        // D#
    PianoKey::WhiteRight,   // E
    PianoKey::WhiteLeft,    // F
    PianoKey::Black,        // F#
    PianoKey::WhiteCenter,  // G
    PianoKey::Black,        // G#
    PianoKey::WhiteCenter,  // A
    PianoKey::Black,        // A#
    PianoKey::WhiteRight    // B
];

fn get_piano_key(index: isize, key_count: isize) -> PianoKey {
    let result = PIANO_KEYS[index.rem_euclid(12) as usize].clone();
    if index >= key_count - 1 && result != PianoKey::Black && result != PianoKey::WhiteRight {
        PianoKey::WhiteFull
    } else {
        result
    }
}

#[derive(Copy, Clone)]
pub struct SliceState {
    pub color: Color,
    pub index: f32,
    pub width: f32,
    pub height: f32,
    pub frame: usize
}

pub struct PianoRollState {
    pub slices: HeapRb<SliceState>,
    samples_per_frame: f32,
    taken_samples: f32,
    starting_octave: f32,
    volume_buf: Vec<f32>,
    frame_count: usize
}

impl PianoRollState {
    pub fn new(sample_rate: f32, scroll_speed: f32, starting_octave: f32) -> Self {
        Self {
            slices: HeapRb::new(APU_STATE_BUF_SIZE),
            samples_per_frame: sample_rate / (60.0 * scroll_speed),
            taken_samples: 0.0,
            starting_octave,
            volume_buf: Vec::new(),
            frame_count: 0
        }
    }

    pub fn consume(&mut self, state: &ChannelState, settings: &ChannelSettings) {
        self.volume_buf.push(state.volume);
        self.taken_samples += 1.0;
        if self.taken_samples < self.samples_per_frame {
            return;
        }
        self.taken_samples -= self.samples_per_frame;

        let n = 12.0 * (state.frequency / C_0).log2() as f32;
        let octave = (n / 12.0).floor() + self.starting_octave;
        let note = n.rem_euclid(12.0);

        let color = settings.color(state).unwrap();
        let index = note + 12.0 * octave;
        let width = self.volume_buf
            .drain(..)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0.0);

        debug_assert!(!index.is_nan(), "Piano key index is NaN?!");

        let mut frame = self.frame_count;

        if let Some(last_slice) = self.slices.iter_mut().last() {
            if last_slice.width == width && ((last_slice.color == color && last_slice.index == index) || width == 0.0) {
                last_slice.height += 1.0;
                return;
            }

            if (last_slice.index - index).abs() < 1.0 && last_slice.color == color && last_slice.width != 0.0 && width != 0.0 {
                frame = last_slice.frame;
            }
        }

        self.slices.push_overwrite(SliceState {
            color,
            index,
            width,
            height: 1.0,
            frame
        });

        self.frame_count += 1;
    }
}

impl Visualizer {
    fn draw_piano_key(&mut self, key: PianoKey, pos: Rect, color: Option<Color>) {
        let key_color = match (color, key) {
            (Some(color), _) => color,
            (None, PianoKey::Black) => Color::BLACK,
            (None, _) => Color::from_rgba8(0x20, 0x20, 0x20, 0xFF)
        };
        let mut key_paint = Paint::default();
        key_paint.anti_alias = false;
        key_paint.set_color(key_color);

        let x = pos.x();
        let y = pos.y();
        let w = pos.width();
        let h = pos.height();
        let w2 = pos.width() / 2.0;
        let h2 = pos.height() / 2.0;

        let mut pb = PathBuilder::new();
        match key {
            PianoKey::WhiteLeft => {
                pb.push_rect(Rect::from_xywh(
                    x - w2 + 1.0,
                    y + 1.0,
                    w - 1.0,
                    h - 1.0
                ).unwrap());
                pb.push_rect(Rect::from_xywh(
                    x + w2,
                    y + h2 + 1.0,
                    w2,
                    h2 - 1.0
                ).unwrap());
            },
            PianoKey::WhiteCenter => {
                pb.push_rect(Rect::from_xywh(
                    x - w2 + 1.0,
                    y + 1.0,
                    w - 1.0,
                    h2
                ).unwrap());
                pb.push_rect(Rect::from_xywh(
                    x - w + 1.0,
                    y + h2 + 1.0,
                    (2.0 * w) - 1.0,
                    h2 - 1.0
                ).unwrap());
            },
            PianoKey::WhiteRight => {
                pb.push_rect(Rect::from_xywh(
                    x - w2 + 1.0,
                    y + 1.0,
                    w - 1.0,
                    h - 1.0
                ).unwrap());
                pb.push_rect(Rect::from_xywh(
                    x - w + 1.0,
                    y + h2 + 1.0,
                    w2,
                    h2 - 1.0
                ).unwrap());
            },
            PianoKey::WhiteFull => {
                pb.push_rect(Rect::from_xywh(
                    x - w2 + 1.0,
                    y + 1.0,
                    w + w2 - 1.0,
                    h - 1.0
                ).unwrap());
            },
            PianoKey::Black => {
                pb.push_rect(Rect::from_xywh(
                    x - w2,
                    y + 1.0,
                    w + 1.0,
                    h2
                ).unwrap());
            }
        }
        let path = pb.finish().unwrap();

        self.canvas.fill_path(
            &path,
            &key_paint,
            FillRule::Winding,
            Transform::identity(),
            None
        );
    }

    fn draw_piano_strings(&mut self, pos: Rect) {
        let mut white_string_paint = Paint::default();
        white_string_paint.anti_alias = false;
        white_string_paint.set_color_rgba8(0x0C, 0x0C, 0x0C, 0xFF);

        let mut black_string_paint = Paint::default();
        black_string_paint.anti_alias = false;
        black_string_paint.set_color_rgba8(0x06, 0x06, 0x06, 0xFF);

        let key_count = 12 * self.config.octave_count as isize + 1;
        let keys_w = self.config.key_thickness * key_count as f32;
        let keys_x = pos.x() + ((pos.width() - keys_w) / 2.0) + (self.config.key_thickness / 2.0) - 1.0;

        for key_i in 0..key_count {
            let string_pos = Rect::from_xywh(
                keys_x + self.config.key_thickness * key_i as f32,
                pos.y(),
                1.0,
                pos.height()
            ).unwrap();

            match get_piano_key(key_i, key_count) {
                PianoKey::Black => self.canvas.fill_rect(
                    string_pos,
                    &black_string_paint,
                    Transform::identity(),
                    None
                ),
                _ => self.canvas.fill_rect(
                    string_pos,
                    &white_string_paint,
                    Transform::identity(),
                    None
                )
            };
        }
    }

    fn draw_piano_keys(&mut self, pos: Rect) {
        let key_count = 12 * self.config.octave_count as isize + 1;

        let keys_w = self.config.key_thickness * key_count as f32;
        let keys_x = pos.x() + ((pos.width() - keys_w) / 2.0) + (self.config.key_thickness / 2.0) - 1.0;

        let mut white_border_paint = Paint::default();
        white_border_paint.anti_alias = false;
        white_border_paint.set_color_rgba8(0x18, 0x18, 0x18, 0xFF);

        let mut top_edge_paint = Paint::default();
        top_edge_paint.anti_alias = false;
        top_edge_paint.set_color_rgba8(0x04, 0x04, 0x04, 0xFF);

        self.canvas.fill_rect(
            Rect::from_xywh(pos.x(), pos.y(), pos.width(), pos.height() + 1.0).unwrap(),
            &top_edge_paint,
            Transform::identity(),
            None
        );
        self.canvas.fill_rect(
            Rect::from_xywh(keys_x, pos.y(), keys_w, pos.height()).unwrap(),
            &white_border_paint,
            Transform::identity(),
            None
        );

        for key_i in 0..key_count {
            let key_t = get_piano_key(key_i, key_count);
            let key_pos = Rect::from_xywh(
                keys_x + self.config.key_thickness * key_i as f32,
                pos.y(),
                self.config.key_thickness,
                pos.height()
            ).unwrap();

            self.draw_piano_key(key_t, key_pos, None);
        }

        self.canvas.fill_rect(
            Rect::from_xywh(pos.x(), pos.y(), pos.width(), 1.0).unwrap(),
            &top_edge_paint,
            Transform::identity(),
            None
        );
    }

    fn draw_channel_key_spot(&mut self, channel: usize, pos: Rect) {
        let key_count = 12 * self.config.octave_count as isize + 1;

        let settings = self.config.settings.settings(channel).unwrap();
        let last_state = self.channel_last_states[channel];

        let color = settings.color(&last_state).unwrap();
        if settings.hidden() || last_state.volume <= 0.0 {
            return;
        }
        let volume_alpha = (0.5 + last_state.volume / 30.0).clamp(0.0, 1.0);

        let n = 12.0 * (last_state.frequency / C_0).log2() as f32;
        let octave = (n / 12.0).floor() + self.config.starting_octave as f32;
        let note = n.rem_euclid(12.0);

        let lower_alpha_multiplier = if note.ceil() != note.floor() {
            note.ceil() - note
        } else {
            1.0
        }.clamp(0.0, 1.0);

        let upper_alpha_multiplier = if note.ceil() != note.floor() {
            note - note.floor()
        } else {
            0.0
        }.clamp(0.0, 1.0);

        let lower_note = note.floor();
        let lower_octave = octave;
        let lower_key = get_piano_key((lower_note + 12.0 * lower_octave) as isize, key_count);
        let lower_alpha = volume_alpha * lower_alpha_multiplier;
        let lower_color = Color::from_rgba(color.red(), color.green(), color.blue(), lower_alpha).unwrap();

        let upper_note = note.ceil().rem_euclid(12.0);
        let upper_octave = octave + (note.ceil() / 12.0).floor();
        let upper_key = get_piano_key((upper_note + 12.0 * upper_octave) as isize, key_count);
        let upper_alpha = volume_alpha * upper_alpha_multiplier;
        let upper_color = Color::from_rgba(color.red(), color.green(), color.blue(), upper_alpha).unwrap();

        let keys_w = self.config.key_thickness * key_count as f32;
        let keys_x = pos.x() + ((pos.width() - keys_w) / 2.0) + (self.config.key_thickness / 2.0) - 1.0;

        let lower_pos = Rect::from_xywh(
            keys_x + self.config.key_thickness * (lower_note + 12.0 * lower_octave),
            pos.y(),
            self.config.key_thickness,
            pos.height()
        ).unwrap();
        let upper_pos = Rect::from_xywh(
            keys_x + self.config.key_thickness * (upper_note + 12.0 * upper_octave),
            pos.y(),
            self.config.key_thickness,
            pos.height()
        ).unwrap();

        self.draw_piano_key(lower_key, lower_pos, Some(lower_color));
        self.draw_piano_key(upper_key, upper_pos, Some(upper_color));
    }

    fn draw_channel_slices(&mut self, pos: Rect, outline: bool) {
        let key_count = 12 * self.config.octave_count as usize + 1;

        let keys_w = self.config.key_thickness * key_count as f32;
        let keys_x = pos.x() + ((pos.width() - keys_w) / 2.0) + (self.config.key_thickness / 2.0) - 1.0;

        let mut slices: Vec<(f32, SliceState)> = Vec::new();

        for channel in 0..self.channels {
            if self.config.settings.settings(channel).unwrap().hidden() {
                continue;
            }

            let mut y = pos.y();
            for slice in self.piano_roll_states[channel].slices.iter().rev() {
                slices.push((y, slice.clone()));

                y += slice.height;
                if y >= pos.bottom() {
                    if let Some((_, last_slice)) = slices.last_mut() {
                        last_slice.height -= y - pos.bottom();
                    }
                    break;
                }
            }
        }

        if !outline {
            slices.sort_unstable_by(|(y_a, slice_a), (y_b, slice_b)| {
                if let Some(width_order) = slice_a.width.partial_cmp(&slice_b.width) {
                    if width_order != Ordering::Equal {
                        // Sort: larger -> smaller
                        return width_order.reverse();
                    }
                }

                if let Some(frame_order) = slice_a.frame.partial_cmp(&slice_b.frame) {
                    if frame_order != Ordering::Equal {
                        // Sort: less recent -> more recent
                        return frame_order.reverse();
                    }
                }

                // Sort: lower -> higher
                y_a.partial_cmp(y_b).unwrap_or(Ordering::Equal)
            });
        }

        for (y, slice) in slices {
            if slice.width > 0.0 {
                let slice_pos: Rect;
                let mut slice_color: Color;

                if outline {
                    slice_pos = Rect::from_xywh(
                        keys_x + (self.config.key_thickness * slice.index) - (slice.width / 2.0) - (self.config.key_thickness / 2.0),
                        y - (self.config.key_thickness / 2.0),
                        slice.width + self.config.key_thickness,
                        slice.height + self.config.key_thickness
                    ).unwrap();
                    slice_color = self.config.outline_color;
                } else {
                    slice_pos = Rect::from_xywh(
                        keys_x + (self.config.key_thickness * slice.index) - (slice.width / 2.0),
                        y,
                        slice.width,
                        slice.height
                    ).unwrap();
                    slice_color = slice.color;
                }

                let mut slice_paint = Paint::default();
                slice_paint.anti_alias = slice.width > 1.0;
                if slice.width < 1.0 {
                    slice_color.set_alpha(slice.width);
                }
                slice_paint.set_color(slice_color);

                self.canvas.fill_rect(
                    slice_pos,
                    &slice_paint,
                    Transform::identity(),
                    None
                );
            }
        }
    }

    pub fn draw_piano_roll(&mut self, pos: Rect) {
        let key_length = self.config.key_length;
        
        let slices_pos = Rect::from_xywh(
            pos.x(),
            pos.y() + key_length,
            pos.width(),
            pos.height() - key_length
        ).unwrap();
        self.draw_channel_slices(slices_pos, true);
        if self.config.draw_piano_strings {
            self.draw_piano_strings(slices_pos);
        }
        self.draw_channel_slices(slices_pos, false);

        let piano_keys_pos = Rect::from_xywh(
            pos.x(),
            pos.y(),
            pos.width(),
            key_length
        ).unwrap();

        self.draw_piano_keys(piano_keys_pos);
        for channel in 0..self.channels {
            self.draw_channel_key_spot(channel, piano_keys_pos);
        }
    }
}
