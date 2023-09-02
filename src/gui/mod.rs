mod render_thread;

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use indicatif::{FormattedDuration, HumanBytes, HumanDuration};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use slint;
use slint::Model as _;
use crate::emulator::ResamplingMode;
use crate::gui::render_thread::{RenderThreadMessage, RenderThreadRequest};
use crate::renderer::render_options::{RendererOptions, StopCondition};
use crate::tuning;

slint::include_modules!();

// The return type looks wrong but it is not
fn slint_string_arr<I>(a: I) -> slint::ModelRc<slint::SharedString>
    where
        I: IntoIterator<Item = String>
{
    let shared_string_vec: Vec<slint::SharedString> = a.into_iter()
        .map(|s| s.into())
        .collect();
    slint::ModelRc::new(slint::VecModel::from(shared_string_vec))
}

fn slint_int_arr<I, N>(a: I) -> slint::ModelRc<i32>
    where
        N: Into<i32>,
        I: IntoIterator<Item = N>
{
    let int_vec: Vec<i32> = a.into_iter()
        .map(|n| n.into())
        .collect();
    slint::ModelRc::new(slint::VecModel::from(int_vec))
}

fn slint_color_component_arr<I: IntoIterator<Item = raqote::Color>>(a: I) -> slint::ModelRc<slint::ModelRc<i32>> {
    let color_vecs: Vec<slint::ModelRc<i32>> = a.into_iter()
        .map(|c| slint::ModelRc::new(slint::VecModel::from(vec![c.r() as i32, c.g() as i32, c.b() as i32])))
        .collect();
    slint::ModelRc::new(slint::VecModel::from(color_vecs))
}

fn browse_for_module_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["spc"])
        .add_filter("SPC files", &["spc"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_background_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["mp4", "mkv", "mov", "avi", "webm", "gif", "jpg", "jpeg", "png", "bmp", "tif", "tiff", "webp", "qoi"])
        .add_filter("Video background formats", &["mp4", "mkv", "mov", "avi", "webm", "gif"])
        .add_filter("Image background formats", &["jpg", "jpeg", "png", "bmp", "tif", "tiff", "webp", "qoi"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_tuning_data() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["json"])
        .add_filter("Super MIDI Pak session files", &["json"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_video_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["mp4", "mkv", "mov"])
        .add_filter("MPEG-4 Video", &["mp4"])
        .add_filter("Matroska Video", &["mkv"])
        .add_filter("QuickTime Video", &["mov"])
        .show_save_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn confirm_prores_export_dialog() -> bool {
    MessageDialog::new()
        .set_title("SPCPresenter")
        .set_text("You have chosen to export a QuickTime video. Do you want to export in ProRes 4444 format to \
                   preserve alpha information for video editing? Note that ProRes 4444 is a lossless codec, so \
                   the exported file may be very large.")
        .set_type(MessageType::Info)
        .show_confirm()
        .unwrap()
}

fn display_error_dialog(text: &str) {
    MessageDialog::new()
        .set_title("SPCPresenter")
        .set_text(text)
        .set_type(MessageType::Error)
        .show_alert()
        .unwrap();
}

fn parse_hex(s: slint::SharedString) -> Option<u8> {
    let parse_result = if s.starts_with('$') {
        u8::from_str_radix(&s[1..], 16)
    } else if s.starts_with("0x") {
        u8::from_str_radix(&s[2..], 16)
    } else {
        return None;
    };

    if parse_result.is_err() {
        return None;
    }

    Some(parse_result.unwrap())
}

fn get_spc_metadata<P: AsRef<Path>>(spc_path: P) -> (bool, Option<Duration>, slint::ModelRc<slint::SharedString>) {
    let (spc_valid, duration, lines) = match spc::spc::Spc::load(spc_path) {
        Ok(spc_file) => {
            match spc_file.id666_tag {
                Some(metadata) => (
                    true,
                    Some(Duration::from_secs(metadata.seconds_to_play_before_fading_out as _)),
                    vec![
                        metadata.song_title,
                        metadata.artist_name,
                        metadata.game_title,
                        metadata.dumper_name
                    ]
                ),
                None => (true, None, vec!["<no metadata>".to_string()])
            }
        },
        _ => (false, None, vec!["<no metadata>".to_string()])
    };

    (spc_valid, duration, slint_string_arr(lines))
}

fn random_slint_color() -> slint::ModelRc<i32> {
    let h = rand::random::<f64>() * 360.0;
    let s = (rand::random::<f64>() * 0.85) + 0.15;
    let l = (rand::random::<f64>() * 0.2) + 0.65;

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (rp, gp, bp) = match h as u32 {
        0 ..= 59 => (c, x, 0.0),
        60 ..= 119 => (x, c, 0.0),
        120 ..= 179 => (0.0, c, x),
        180 ..= 239 => (0.0, x, c),
        240 ..= 299 => (x, 0.0, c),
        300 ..= 359 => (c, 0.0, x),
        _ => unreachable!()
    };

    let r = ((rp + m) * 255.0) as u8;
    let g = ((gp + m) * 255.0) as u8;
    let b = ((bp + m) * 255.0) as u8;

    slint_int_arr([r as i32, g as i32, b as i32])
}

pub fn run() {
    let main_window = MainWindow::new().unwrap();

    main_window.global::<ColorUtils>().on_hex_to_color(|hex| {
        let rgb = u32::from_str_radix(hex.to_string().trim_start_matches("#"), 16).unwrap_or(0);

        slint::Color::from_argb_encoded(0xFF000000 | rgb)
    });

    main_window.global::<ColorUtils>().on_color_to_hex(|color| {
        format!("#{:02x}{:02x}{:02x}", color.red(), color.green(), color.blue()).into()
    });

    main_window.global::<ColorUtils>().on_color_components(|color| {
        slint_int_arr([color.red() as i32, color.green() as i32, color.blue() as i32])
    });

    main_window.global::<SampleUtils>().on_is_hex(|s| {
        parse_hex(s).is_some()
    });

    main_window.global::<SampleUtils>().on_parse_hex(|s| {
        parse_hex(s).unwrap_or(0) as i32
    });

    main_window.global::<SampleUtils>().on_format_hex(|i| {
        format!("${:02x}", i).into()
    });

    let mut options = Rc::new(RefCell::new(RendererOptions::default()));

    let (rt_handle, rt_tx) = {
        let main_window_weak = main_window.as_weak();
        render_thread::render_thread(move |msg| {
            match msg {
                RenderThreadMessage::Error(e) => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_error(true);
                        main_window_weak.unwrap().set_progress_status(format!("Render error: {}", e).into());

                    }).unwrap();
                }
                RenderThreadMessage::RenderStarting => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(true);
                        main_window_weak.unwrap().set_progress_error(false);
                        main_window_weak.unwrap().set_progress(0.0);
                        main_window_weak.unwrap().set_progress_title("Rendering".into());
                        main_window_weak.unwrap().set_progress_status("Setting up".into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderProgress(p) => {
                    let current_video_size = HumanBytes(p.encoded_size as u64);
                    let current_video_duration = FormattedDuration(p.encoded_duration);
                    let expected_video_duration = match p.expected_duration {
                        Some(duration) => FormattedDuration(duration).to_string(),
                        None => "(unknown)".to_string()
                    };
                    // let elapsed_duration = FormattedDuration(p.elapsed_duration);
                    let eta_duration = match p.eta_duration {
                        Some(duration) => HumanDuration(duration.saturating_sub(p.elapsed_duration)).to_string(),
                        None => "Unknown time".to_string()
                    };

                    let (progress, progress_title) = match p.expected_duration_frames {
                        Some(exp_dur_frames) => {
                            let progress = p.frame as f64 / exp_dur_frames as f64;
                            (progress, "Rendering".to_string())
                        },
                        None => (0.0, "Initializing".to_string()),
                    };
                    let progress_status = format!(
                        "{}%, {} FPS, encoded {}/{} ({}), {} remaining",
                        (progress * 100.0).round(),
                        p.average_fps,
                        current_video_duration, expected_video_duration,
                        current_video_size,
                        eta_duration
                    );

                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_progress(progress as f32);
                        main_window_weak.unwrap().set_progress_title(progress_title.into());
                        main_window_weak.unwrap().set_progress_status(progress_status.into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderComplete => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress(1.0);
                        main_window_weak.unwrap().set_progress_title("Idle".into());
                        main_window_weak.unwrap().set_progress_status("Finished".into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderCancelled => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_title("Idle".into());
                        main_window_weak.unwrap().set_progress_status("Render cancelled".into());
                    }).unwrap();
                }
            }
        })
    };

    {
        let main_window_weak = main_window.as_weak();
        let mut options = options.clone();
        main_window.on_browse_for_module(move || {
            match browse_for_module_dialog() {
                Some(path) => {
                    let (spc_valid, _duration, metadata_lines) = get_spc_metadata(&path);

                    if !spc_valid {
                        display_error_dialog("Invalid SPC file.");
                        return options.borrow().input_path.clone().into();
                    }

                    options.borrow_mut().input_path = path.clone();
                    main_window_weak.unwrap().set_metadata_lines(metadata_lines);

                    main_window_weak.unwrap().invoke_reformat_duration();

                    path.into()
                },
                None => options.borrow().input_path.clone().into()
            }
        });
    }

    {
        let mut options = options.clone();
        main_window.on_browse_for_background(move || {
            match browse_for_background_dialog() {
                Some(path) => {
                    options.borrow_mut().video_options.background_path = Some(path.clone().into());

                    path.into()
                },
                None => options.borrow().video_options.background_path.clone().unwrap_or("".to_string()).into()
            }
        });
    }

    {
        let mut options = options.clone();
        main_window.on_background_cleared(move || {
            options.borrow_mut().video_options.background_path = None;
        });
    }

    {
        let mut options = options.clone();
        main_window.on_format_duration(move |stop_condition_type, stop_condition_num| {
            let duration = match stop_condition_type {
                StopConditionType::Frames => {
                    let seconds = (stop_condition_num as f64) / 60.0;
                    Duration::from_secs_f64(seconds)
                },
                StopConditionType::Time => {
                    Duration::from_secs(stop_condition_num as _)
                },
                StopConditionType::SpcDuration => {
                    let (_spc_valid, duration, _metadata_lines) = get_spc_metadata(options.borrow().input_path.clone());
                    if duration.is_none() {
                        return "<error>".into();
                    }
                    duration.unwrap()
                }
            };
            FormattedDuration(duration).to_string().into()
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_import_tunings(move || {
            let tuning_data_path = match browse_for_tuning_data() {
                Some(path) => path,
                None => return
            };

            let new_samples: Vec<SampleConfig> = {
                if tuning_data_path.ends_with(".json") {
                    let session_json = match fs::read_to_string(tuning_data_path) {
                        Ok(json) => json,
                        Err(e) => {
                            display_error_dialog(&format!("Failed to read tuning data: {}", e));
                            return;
                        }
                    };

                    let session = match tuning::super_midi_pak_session::SuperMidiPakSession::from_json(&session_json) {
                        Ok(session) => session,
                        Err(e) => {
                            display_error_dialog(&format!("Failed to parse tuning data: {}", e));
                            return;
                        }
                    };

                    match session.samples() {
                        Ok(samples) => {
                            samples.iter()
                                .map(|s| {
                                    let (pitch_type, frequency) = match s.pitch {
                                        Some(pitch) => (PitchType::Frequency, pitch as f32),
                                        None => (PitchType::Automatic, 500.0)
                                    };

                                    SampleConfig {
                                        name: s.name.clone().into(),
                                        source: s.source as i32,
                                        pitch_type,
                                        frequency,
                                        amk_tuning: 3,
                                        amk_subtuning: 0,
                                        color: random_slint_color(),
                                        use_color: false
                                    }
                                })
                                .collect()
                        },
                        Err(e) => {
                            display_error_dialog(&format!("Failed to parse tuning data: {}", e));
                            return;
                        }
                    }
                } else {
                    vec![]
                }
            };

            let mut sample_configs: Vec<SampleConfig> = main_window_weak.unwrap().get_sample_configs()
                .iter()
                .filter(|c| !new_samples.iter().any(|n| n.source == c.source))
                .collect();
            sample_configs.extend(new_samples);
            main_window_weak.unwrap().set_sample_configs(slint::ModelRc::new(slint::VecModel::from(sample_configs)));
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_add_sample(move |source| {
            let new_sample = SampleConfig {
                name: "<none>".into(),
                source,
                pitch_type: PitchType::Frequency,
                frequency: 500.0,
                amk_tuning: 3,
                amk_subtuning: 0,
                color: random_slint_color(),
                use_color: false
            };

            let mut sample_configs: Vec<SampleConfig> = main_window_weak.unwrap().get_sample_configs().iter().collect();
            sample_configs.push(new_sample);

            let result = (sample_configs.len() - 1) as i32;

            main_window_weak.unwrap().set_sample_configs(slint::ModelRc::new(slint::VecModel::from(sample_configs)));

            result
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        main_window.on_remove_sample(move |index| {
            let mut sample_configs: Vec<SampleConfig> = main_window_weak.unwrap().get_sample_configs().iter().collect();

            if index < 0 || index >= sample_configs.len() as i32 {
                return -1;
            }

            sample_configs.remove(index as usize);

            let result = if sample_configs.len() > index as usize {
                (sample_configs.len() - 1) as i32
            } else {
                // This may return -1 for an empty list but that's OK since
                // -1 is a sentinel for "no item selected"
                index - 1
            };

            main_window_weak.unwrap().set_sample_configs(slint::ModelRc::new(slint::VecModel::from(sample_configs)));

            result
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let mut options = options.clone();
        let rt_tx = rt_tx.clone();
        main_window.on_start_render(move || {
            let output_path = match browse_for_video_dialog() {
                Some(path) => path,
                None => return
            };

            if output_path.ends_with(".mov") && confirm_prores_export_dialog() {
                // -c:v prores_ks -profile:v 4 -bits_per_mb 1000 -pix_fmt yuva444p10le
                options.borrow_mut().video_options.video_codec = "prores_ks".to_string();
                options.borrow_mut().video_options.video_codec_params.insert("profile".to_string(), "4".to_string());
                options.borrow_mut().video_options.video_codec_params.insert("bits_per_mb".to_string(), "1000".to_string());
                options.borrow_mut().video_options.pixel_format_out = "yuva444p10le".to_string();
            }

            options.borrow_mut().video_options.output_path = output_path;
            options.borrow_mut().fadeout_length = main_window_weak.unwrap().get_fadeout_duration() as u64;
            options.borrow_mut().video_options.resolution_out.0 = main_window_weak.unwrap().get_output_width() as u32;
            options.borrow_mut().video_options.resolution_out.1 = main_window_weak.unwrap().get_output_height() as u32;

            let stop_condition_num = main_window_weak.unwrap().get_stop_condition_num() as u64;
            options.borrow_mut().stop_condition = match main_window_weak.unwrap().get_stop_condition_type() {
                StopConditionType::Frames => StopCondition::Frames(stop_condition_num),
                StopConditionType::Time => StopCondition::Frames(stop_condition_num * 60),
                StopConditionType::SpcDuration => StopCondition::SpcDuration
            };

            let base_colors: Vec<raqote::Color> = main_window_weak.unwrap().get_channel_base_colors()
                .as_any()
                .downcast_ref::<slint::VecModel<slint::ModelRc<i32>>>()
                .unwrap()
                .iter()
                .map(|color_model| {
                    let mut component_iter = color_model
                        .as_any()
                        .downcast_ref::<slint::VecModel<i32>>()
                        .unwrap()
                        .iter();
                    let r = component_iter.next().unwrap() as u8;
                    let g = component_iter.next().unwrap() as u8;
                    let b = component_iter.next().unwrap() as u8;

                    raqote::Color::new(0xFF, r, g, b)
                })
                .collect();
            options.borrow_mut().channel_base_colors = base_colors;

            let sample_configs: Vec<SampleConfig> = main_window_weak.unwrap().get_sample_configs()
                .as_any()
                .downcast_ref::<slint::VecModel<SampleConfig>>()
                .unwrap()
                .iter()
                .collect();
            for config in sample_configs {
                match config.pitch_type {
                    PitchType::Automatic => (),
                    PitchType::Frequency => {
                        options.borrow_mut().manual_sample_tunings.insert(config.source as u8, config.frequency as f64);
                    },
                    PitchType::AddMusicK => {
                        let frequency = 32000.0 / (16.0 * ((config.amk_tuning as f64) + (config.amk_subtuning as f64 / 256.0)));
                        options.borrow_mut().manual_sample_tunings.insert(config.source as u8, frequency);
                    }
                }

                if config.use_color {
                    let color = {
                        let mut component_iter = config.color
                            .as_any()
                            .downcast_ref::<slint::VecModel<i32>>()
                            .unwrap()
                            .iter();
                        let r = component_iter.next().unwrap() as u8;
                        let g = component_iter.next().unwrap() as u8;
                        let b = component_iter.next().unwrap() as u8;

                        raqote::Color::new(0xFF, r, g, b)
                    };
                    options.borrow_mut().per_sample_colors.insert(config.source as u8, color);
                }
            }

            if options.borrow().video_options.background_path.clone().unwrap_or("".to_string()).is_empty() {
                options.borrow_mut().video_options.background_path = None;
            }

            options.borrow_mut().filter_enabled = main_window_weak.unwrap().get_filter_enabled();
            options.borrow_mut().resampling_mode = match main_window_weak.unwrap().get_accurate_interp() {
                true => ResamplingMode::AccurateGaussian,
                false => ResamplingMode::Gaussian
            };

            rt_tx.send(RenderThreadRequest::StartRender(options.borrow().clone())).unwrap();
        });
    }

    {
        let rt_tx = rt_tx.clone();
        main_window.on_cancel_render(move || {
            rt_tx.send(RenderThreadRequest::CancelRender).unwrap();
        });
    }

    main_window.run().unwrap();

    if rt_tx.send(RenderThreadRequest::Terminate).is_ok() {
        // If the send failed, the channel is closed, so the thread is probably already dead.
        rt_handle.join().unwrap();
    }
}
