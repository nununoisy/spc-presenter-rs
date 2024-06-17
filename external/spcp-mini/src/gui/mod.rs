mod audio;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use slint;
use native_dialog::FileDialog;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use slint::Model;
use crate::emulator::Emulator;

slint::include_modules!();

fn browse_for_spc() -> Option<PathBuf> {
    let file_dialog = FileDialog::new()
        .set_title("Open Song")
        .add_filter("SPC Files", &["spc"]);

    match file_dialog.show_open_single_file() {
        Ok(Some(path)) => Some(path),
        _ => None
    }
}

pub fn run() {
    let main_window = MainWindow::new().unwrap();

    let mut emulator = Emulator::new();

    let source = emulator.iter();
    let audio_manager = Arc::new(Mutex::new(audio::AudioManager::new()));

    {
        let source = source.clone();
        let mut audio_manager = audio_manager.clone();
        main_window.on_init_audio(move || {
            audio_manager.lock().unwrap().init(source.clone(), None);
        });
    }

    {
        main_window.on_format_duration(move |duration| {
            let raw_seconds = duration / 1000;
            let minutes = raw_seconds / 60;
            let seconds = raw_seconds % 60;
            format!("{}:{:02}", minutes, seconds).into()
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let audio_manager = audio_manager.clone();
        main_window.on_open_spc(move || {
            let spc_path = match browse_for_spc() {
                Some(path) => path,
                None => return
            };
            if let Some(metadata) = emulator.load_spc(spc_path) {
                main_window_weak.unwrap().set_spc_title(metadata.song_title.into());
                main_window_weak.unwrap().set_spc_artist(metadata.artist_name.into());
                main_window_weak.unwrap().set_spc_game(metadata.game_title.into());
                main_window_weak.unwrap().set_spc_ripper(metadata.dumper_name.into());
                main_window_weak.unwrap().set_spc_duration(metadata.play_time.as_millis() as i64);
                main_window_weak.unwrap().set_spc_fadeout(metadata.fadeout_time.as_millis() as i64);
            }

            main_window_weak.unwrap().invoke_init_audio();

            let audio_manager = audio_manager.lock().unwrap();
            audio_manager.play();
            main_window_weak.unwrap().set_playing(!audio_manager.is_paused());
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let source = source.clone();
        let audio_manager = audio_manager.clone();
        main_window.on_play_pause(move || {
            let audio_manager = audio_manager.lock().unwrap();
            if !main_window_weak.unwrap().get_playing() || main_window_weak.unwrap().get_seeking() {
                audio_manager.pause();
            } else {
                let playback_position = source.position();
                let playback_duration = Duration::from_millis(main_window_weak.unwrap().get_playback_duration() as u64);
                if !playback_duration.is_zero() && playback_position > playback_duration {
                    audio_manager.seek(Duration::ZERO);
                }
                audio_manager.play();
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let audio_manager = audio_manager.clone();
        main_window.on_seek(move |pos| {
            let audio_manager = audio_manager.lock().unwrap();
            audio_manager.seek(Duration::from_millis(pos as u64));
            // Force buffering state to refresh Slider element
            main_window_weak.unwrap().set_buffer_length(0);
        });
    }

    let updater = slint::Timer::default();
    {
        let main_window_weak = main_window.as_weak();
        let source = source.clone();
        let audio_manager = audio_manager.clone();
        updater.start(slint::TimerMode::Repeated, Duration::from_millis(10), move || {
            let playback_position = source.position();
            let buffer_length = source.buffer_length();
            if !main_window_weak.unwrap().get_seeking() {
                main_window_weak.unwrap().set_playback_position(playback_position.as_millis() as i64);
                main_window_weak.unwrap().set_buffer_length(buffer_length.as_millis() as i64);
            }

            let audio_manager = audio_manager.lock().unwrap();
            let fadeout_start = Duration::from_millis(main_window_weak.unwrap().get_fadeout_start() as u64);
            let playback_duration = Duration::from_millis(main_window_weak.unwrap().get_playback_duration() as u64);
            if !main_window_weak.unwrap().get_repeat_infinite() && playback_position > playback_duration {
                if main_window_weak.unwrap().get_repeat() && main_window_weak.unwrap().get_playing() {
                    audio_manager.seek(Duration::ZERO);
                } else {
                    audio_manager.pause();
                    main_window_weak.unwrap().set_playing(false);
                }
            } else if !main_window_weak.unwrap().get_repeat_infinite() && playback_position > fadeout_start {
                let fadeout_duration = (playback_duration - fadeout_start).as_secs_f32();
                if fadeout_duration != 0.0 {
                    let fadeout_position = ((fadeout_duration - (playback_duration - playback_position).as_secs_f32()) / fadeout_duration).clamp(0.0, 1.0);
                    audio_manager.set_volume(1.0 - fadeout_position.powi(3));
                }
            } else if playback_position >= buffer_length {
                // Don't be noisy if we need to buffer
                audio_manager.set_volume(0.0);
            } else {
                audio_manager.set_volume(1.0);
            }
            drop(audio_manager);

            if let Some(emulator_state) = source.apu_state() {
                let apu_channel_states = main_window_weak.unwrap().get_apu_channel_states();
                for (i, state) in emulator_state.iter().enumerate() {
                    apu_channel_states.set_row_data(i, SlintApuChannelState {
                        echo_on: state.echo_delay.is_some(),
                        envelope: state.envelope_level,
                        noise_on: state.noise_clock.is_some(),
                        output_left: state.amplitude.0,
                        output_right: state.amplitude.1,
                        pitch: state.pitch as i32,
                        pitch_modulation_on: state.pitch_modulation,
                        volume_left: state.volume.0 as i32,
                        volume_right: state.volume.1 as i32,
                    });
                }
                {
                    let state = emulator_state.master();
                    main_window_weak.unwrap().set_apu_master_state(SlintApuMasterState {
                        master_volume_left: state.master_volume.0 as i32,
                        master_volume_right: state.master_volume.1 as i32,
                        echo_volume_left: state.echo_volume.0 as i32,
                        echo_volume_right: state.echo_volume.1 as i32,
                        echo_delay: state.echo_delay as i32,
                        echo_feedback: state.echo_feedback as i32,
                        output_left: state.amplitude.0,
                        output_right: state.amplitude.1
                    });
                }
            }
        });
    }

    main_window.run().unwrap();
}