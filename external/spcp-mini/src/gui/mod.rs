mod audio;

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use slint;
use native_dialog::FileDialog;
use rodio::Source;
use slint::Model;
use i_slint_backend_winit::{WinitWindowAccessor, winit::window::ResizeDirection};
use i_slint_backend_winit::winit::raw_window_handle::{HasRawWindowHandle, HasWindowHandle, RawWindowHandle};
use i_slint_backend_winit::winit::window::UserAttentionType;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig, SeekDirection};
use crate::emulator::Emulator;

slint::include_modules!();

#[cfg(not(target_os = "windows"))]
unsafe fn get_hwnd(_window: &slint::Window) -> Option<*mut c_void> {
    None
}


#[cfg(target_os = "windows")]
unsafe fn get_hwnd(window: &slint::Window) -> Option<*mut c_void> {
    match window.window_handle().raw_window_handle().ok()? {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut c_void),
        _ => None
    }
}

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

    let controls = Arc::new(Mutex::new(MediaControls::new(PlatformConfig {
        dbus_name: "spcp_mini",
        display_name: "SPCPresenter Mini",
        hwnd: unsafe { get_hwnd(main_window.window()) }
    }).unwrap()));

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
        main_window.on_start_window_drag(move || {
            main_window_weak.unwrap().window().with_winit_window(|winit_window| {
                let _ = winit_window.drag_window();
            });
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_start_window_resize(move |n, e, s, w| {
            let direction = match (n, e, s, w) {
                (true, true, false, false) => ResizeDirection::NorthEast,
                (true, false, false, true) => ResizeDirection::NorthWest,
                (true, false, false, false) => ResizeDirection::North,
                (false, true, true, false) => ResizeDirection::SouthEast,
                (false, false, true, true) => ResizeDirection::SouthWest,
                (false, false, true, false) => ResizeDirection::South,
                (false, true, false, false) => ResizeDirection::East,
                (false, false, false, true) => ResizeDirection::West,

                _ => return
            };
            main_window_weak.unwrap().window().with_winit_window(|winit_window| {
                let _ = winit_window.drag_resize_window(direction);
            });
        })
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_minimize_pressed(move || {
            main_window_weak.unwrap().window().set_minimized(true);
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_maximize_pressed(move || {
            let new_maximized = !main_window_weak.unwrap().window().is_maximized();
            main_window_weak.unwrap().window().set_maximized(new_maximized);
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_close_pressed(move || {
            main_window_weak.unwrap().hide().unwrap();
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let audio_manager = audio_manager.clone();
        let controls = controls.clone();
        main_window.on_open_spc(move || {
            let spc_path = match browse_for_spc() {
                Some(path) => path,
                None => return
            };
            if let Some(spc) = emulator.load_spc(spc_path) {
                let metadata = spc.metadata();

                main_window_weak.unwrap().set_spc_title(metadata.song_title().unwrap_or_default().into());
                main_window_weak.unwrap().set_spc_artist(metadata.artist_name().unwrap_or_default().into());
                main_window_weak.unwrap().set_spc_game(metadata.game_title().unwrap_or_default().into());
                main_window_weak.unwrap().set_spc_ripper(metadata.dumper_name().unwrap_or_default().into());

                let (play_time, fadeout_time) = metadata.play_time(None)
                    .unwrap_or((Duration::from_secs(300), Duration::from_secs(10)));

                main_window_weak.unwrap().set_spc_duration(play_time.as_millis() as i64);
                main_window_weak.unwrap().set_spc_fadeout(fadeout_time.as_millis() as i64);

                controls.lock().unwrap().set_metadata(MediaMetadata {
                    title: Some(metadata.song_title().unwrap_or_default().as_str()),
                    artist: Some(metadata.artist_name().unwrap_or_default().as_str()),
                    album: Some(metadata.game_title().unwrap_or_default().as_str()),
                    duration: Some(play_time + fadeout_time),
                    ..Default::default()
                }).unwrap();
            } else {
                controls.lock().unwrap().set_metadata(MediaMetadata::default()).unwrap();
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
        let controls = controls.clone();
        updater.start(slint::TimerMode::Repeated, Duration::from_millis(10), move || {
            let maximized = main_window_weak.unwrap().window().is_maximized();
            main_window_weak.unwrap().set_maximized(maximized);

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

            if main_window_weak.unwrap().window().is_visible() {
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
            }

            let playback = if main_window_weak.unwrap().get_playing() {
                MediaPlayback::Playing {
                    progress: Some(MediaPosition(playback_position))
                }
            } else {
                MediaPlayback::Paused {
                    progress: Some(MediaPosition(playback_position))
                }
            };
            controls.lock().unwrap().set_playback(playback).unwrap();
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let source = source.clone();
        let audio_manager = audio_manager.clone();
        controls.lock().unwrap().attach(move |event| {
            match event {
                MediaControlEvent::Pause => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    main_window.set_playing(false);
                    main_window.invoke_play_pause();
                }).unwrap(),
                MediaControlEvent::Play => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    main_window.set_playing(true);
                    main_window.invoke_play_pause();
                }).unwrap(),
                MediaControlEvent::Toggle => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    main_window.set_playing(!main_window.get_playing());
                    main_window.invoke_play_pause();
                }).unwrap(),
                MediaControlEvent::Previous => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    main_window.invoke_seek(0);
                }).unwrap(),
                MediaControlEvent::Quit => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    main_window.invoke_close_pressed();
                }).unwrap(),
                MediaControlEvent::Raise => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    let _ = main_window.window().with_winit_window(|winit_window| {
                        winit_window.focus_window();
                        winit_window.request_user_attention(Some(UserAttentionType::Informational));
                    });
                }).unwrap(),
                MediaControlEvent::Seek(direction) => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    let mut position = Duration::from_millis(main_window.get_playback_position() as u64);
                    match direction {
                        SeekDirection::Forward => {
                            position += Duration::from_secs(5);
                            if !main_window.get_repeat_infinite() {
                                let playback_duration = Duration::from_millis(main_window.get_playback_duration() as u64);
                                position = position.min(playback_duration);
                            }
                        },
                        SeekDirection::Backward => {
                            position = position.saturating_sub(Duration::from_secs(5));
                        }
                    }
                    main_window.invoke_seek(position.as_millis() as i64);
                }).unwrap(),
                MediaControlEvent::SeekBy(direction, amount) => main_window_weak.upgrade_in_event_loop(move |main_window| {
                    let mut position = Duration::from_millis(main_window.get_playback_position() as u64);
                    match direction {
                        SeekDirection::Forward => {
                            position += amount;
                            if !main_window.get_repeat_infinite() {
                                let playback_duration = Duration::from_millis(main_window.get_playback_duration() as u64);
                                position = position.min(playback_duration);
                            }
                        },
                        SeekDirection::Backward => {
                            position = position.saturating_sub(amount);
                        }
                    }
                    main_window.invoke_seek(position.as_millis() as i64);
                }).unwrap(),

                _ => ()
            }
        }).unwrap();
    }

    main_window.run().unwrap();
}