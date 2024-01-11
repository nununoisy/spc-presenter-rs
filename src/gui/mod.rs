mod render_thread;
mod sample_processing_thread;
mod audio_previewer;
mod localization;

use std::sync::{Arc, Mutex};
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::env;
use anyhow::Result;
use native_dialog::{FileDialog, MessageDialog, MessageType};
use slint;
use slint::Model as _;
use tiny_skia::Color;
use render_thread::{RenderThreadMessage, RenderThreadRequest};
use sample_processing_thread::{SampleProcessingThreadMessage, SampleProcessingThreadRequest};
use audio_previewer::{AudioPreviewer, audio_stopped_timer};
use localization::fluent_args;
use fluent::FluentArgs;
use crate::config::Config;
use crate::emulator::ResamplingMode;
use crate::renderer::render_options::{RendererOptions, StopCondition};
use crate::sample_processing::SampleProcessorProgress;
use crate::tuning;

slint::include_modules!();

// The return type looks wrong but it is not
fn slint_string_arr<I>(a: I) -> slint::ModelRc<slint::SharedString>
    where
        I: IntoIterator,
        I::Item: Into<slint::SharedString>
{
    let shared_string_vec: Vec<slint::SharedString> = a.into_iter()
        .map(|s| s.into())
        .collect();
    slint::ModelRc::new(slint::VecModel::from(shared_string_vec))
}

fn slint_int_arr<I>(a: I) -> slint::ModelRc<i32>
    where
        I: IntoIterator,
        I::Item: Into<i32>
{
    let int_vec: Vec<i32> = a.into_iter()
        .map(|n| n.into())
        .collect();
    slint::ModelRc::new(slint::VecModel::from(int_vec))
}

fn slint_color_component_arr<I: IntoIterator<Item = Color>>(a: I) -> slint::ModelRc<slint::ModelRc<i32>> {
    let color_vecs: Vec<slint::ModelRc<i32>> = a.into_iter()
        .map(|c| c.to_color_u8())
        .map(|c| slint::ModelRc::new(slint::VecModel::from(vec![c.red() as i32, c.green() as i32, c.blue() as i32])))
        .collect();
    slint::ModelRc::new(slint::VecModel::from(color_vecs))
}

fn slint_duration(duration: Duration) -> i64 {
    duration.as_millis() as i64
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

fn browse_for_config_import_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("Configuration File", &["toml"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_config_export_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("Configuration File", &["toml"])
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

fn browse_for_dump_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("BRR samples", &["brr"])
        .show_save_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn display_error_dialog(text: &str) {
    MessageDialog::new()
        .set_title("SPCPresenter")
        .set_text(text.replace('\u{2068}', "").replace('\u{2069}', "").as_str())
        .set_type(MessageType::Error)
        .show_alert()
        .unwrap();
}

fn parse_hex(s: slint::SharedString) -> Option<u16> {
    if s.starts_with('$') {
        u16::from_str_radix(&s[1..], 16).ok()
    } else if s.starts_with("0x") {
        u16::from_str_radix(&s[2..], 16).ok()
    } else {
        None
    }
}

fn get_spc_metadata<P: AsRef<Path>>(spc_path: P) -> Result<(Option<Duration>, slint::ModelRc<slint::SharedString>)> {
    let spc_file = spc::spc::Spc::load(spc_path)?;

    let (duration, lines) = match spc_file.id666_tag {
        Some(metadata) => (
            Some(metadata.play_time),
            vec![
                metadata.song_title,
                metadata.artist_name,
                metadata.game_title,
                metadata.dumper_name
            ]
        ),
        None => (None, vec![])
    };

    Ok((duration, slint_string_arr(lines)))
}

fn random_slint_color() -> slint::ModelRc<i32> {
    let h = rand::random::<f64>() * 360.0;
    let s = (rand::random::<f64>() * 0.25) + 0.75;
    let v = (rand::random::<f64>() * 0.15) + 0.85;

    let rgb: Vec<i32> = (0..3)
        .map(|i| {
            let k = ((5.0 - (2.0 * i as f64)) + (h / 60.0)) % 6.0;
            let c = v - (v * s * k.min(4.0 - k).clamp(0.0, 1.0));
            (c * 255.0).floor() as i32
        })
        .collect();

    slint_int_arr(rgb)
}

const UNKNOWN_DURATION: i64 = -2;
const ERROR_DURATION: i64 = -1;

fn default_progress_info(progress_type: ProgressType) -> ProgressInfo {
    let mut result = ProgressInfo {
        progress_type,
        progress: 0.0,
        error: "".into(),
        fps: 0,
        encoded_duration: UNKNOWN_DURATION,
        expected_duration: UNKNOWN_DURATION,
        video_size: 0,
        eta: UNKNOWN_DURATION,
        source: 0,
        current_sample: 0,
        total_samples: 0
    };

    match progress_type {
        ProgressType::RenderError | ProgressType::ProcessorError => result.progress = 1.0,
        ProgressType::RenderFinished | ProgressType::ProcessorFinished => result.progress = 1.0,
        ProgressType::RenderCancelled | ProgressType::ProcessorCancelled => result.progress = 1.0,
        _ => ()
    }

    result
}

fn slint_optional_duration(duration: Option<Duration>) -> i64 {
    duration.map(slint_duration).unwrap_or(UNKNOWN_DURATION)
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

    let localization_adapter = Arc::new(Mutex::new(localization::LocalizationAdapter::new()));
    if let Ok(language) = env::var("PRESENTER_LANG") {
        localization_adapter.lock().unwrap().set_language(&language);
    }

    {
        let localization_adapter = localization_adapter.clone();
        main_window.global::<Localization>().on_tr(move |message_id| {
            let localization_adapter = localization_adapter.lock().unwrap();
            localization_adapter.get(message_id.as_str(), None).into()
        });
    }

    {
        let localization_adapter = localization_adapter.clone();
        main_window.global::<Localization>().on_tr_args(move |message_id, slint_args| {
            let localization_adapter = localization_adapter.lock().unwrap();

            let mut args = FluentArgs::new();
            for slint_arg in slint_args.as_any().downcast_ref::<slint::VecModel<LocalizationArg>>().unwrap().iter() {
                if slint_arg.is_int {
                    args.set(slint_arg.id.to_string(), slint_arg.i_value);
                } else {
                    args.set(slint_arg.id.to_string(), slint_arg.s_value.to_string());
                }
            }

            localization_adapter.get(message_id.as_str(), Some(&args)).into()
        });
    }

    main_window.set_version(env!("CARGO_PKG_VERSION").into());
    main_window.set_ffmpeg_version(crate::video_builder::ffmpeg_version().into());

    let options = Arc::new(Mutex::new(RendererOptions::default()));

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_update_config(move |write_to_config| {
            let config = &mut options.lock().unwrap().config;

            if write_to_config {
                main_window_weak.unwrap().get_channel_base_colors()
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

                        Color::from_rgba8(r, g, b, 0xFF)
                    })
                    .enumerate()
                    .for_each(|(channel, color)| {
                        config.piano_roll
                            .settings
                            .settings_mut(channel)
                            .unwrap()
                            .set_colors(&[color]);
                    });

                config.emulator.filter_enabled = main_window_weak.unwrap().get_filter_enabled();
                config.emulator.resampling_mode = match main_window_weak.unwrap().invoke_resampling_type() {
                    ResamplingType::Accurate => ResamplingMode::Accurate,
                    ResamplingType::Gaussian => ResamplingMode::Gaussian,
                    ResamplingType::Linear => ResamplingMode::Linear,
                    ResamplingType::Cubic => ResamplingMode::Cubic,
                    ResamplingType::Sinc => ResamplingMode::Sinc
                };
            } else {
                let base_colors: Vec<Color> = (0..8)
                    .map(|channel| {
                        config.piano_roll
                            .settings
                            .settings(channel)
                            .unwrap()
                            .colors()[0]
                    })
                    .collect();
                main_window_weak.unwrap().set_channel_base_colors(slint_color_component_arr(base_colors));

                main_window_weak.unwrap().set_filter_enabled(config.emulator.filter_enabled);
                main_window_weak.unwrap().invoke_set_resampling_type(match &config.emulator.resampling_mode {
                    ResamplingMode::Accurate => ResamplingType::Accurate,
                    ResamplingMode::Gaussian => ResamplingType::Gaussian,
                    ResamplingMode::Linear => ResamplingType::Linear,
                    ResamplingMode::Cubic => ResamplingType::Cubic,
                    ResamplingMode::Sinc => ResamplingType::Sinc
                });
            }
        });
    }
    main_window.invoke_update_config(false);

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        let localization_adapter = localization_adapter.clone();
        main_window.on_import_config(move || {
            match browse_for_config_import_dialog() {
                Some(path) => {
                    let new_config_str = match fs::read_to_string(path) {
                        Ok(d) => d,
                        Err(e) => {
                            let message = localization_adapter
                                .lock()
                                .unwrap()
                                .get("error-message-config-read-error", Some(&fluent_args!(error: e.to_string())));
                            display_error_dialog(&message);
                            return;
                        }
                    };
                    options.lock().unwrap().config = match Config::from_toml(&new_config_str) {
                        Ok(c) => c,
                        Err(e) => {
                            let message = localization_adapter
                                .lock()
                                .unwrap()
                                .get("error-message-config-parse-error", Some(&fluent_args!(error: e.to_string())));
                            display_error_dialog(&message);
                            return;
                        }
                    };
                    main_window_weak.unwrap().invoke_update_config(false);
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        let localization_adapter = localization_adapter.clone();
        main_window.on_export_config(move || {
            match browse_for_config_export_dialog() {
                Some(path) => {
                    main_window_weak.unwrap().invoke_update_config(true);

                    let config_str = match options.lock().unwrap().config.export() {
                        Ok(c) => c,
                        Err(e) => {
                            let message = localization_adapter
                                .lock()
                                .unwrap()
                                .get("error-message-config-serialize-error", Some(&fluent_args!(error: e.to_string())));
                            display_error_dialog(&message);
                            return;
                        }
                    };

                    match fs::write(&path, config_str) {
                        Ok(()) => (),
                        Err(e) => {
                            let message = localization_adapter
                                .lock()
                                .unwrap()
                                .get("error-message-config-write-error", Some(&fluent_args!(error: e.to_string())));
                            display_error_dialog(&message);
                            return;
                        }
                    }
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_reset_config(move || {
            options.lock().unwrap().config = Config::default();
            main_window_weak.unwrap().invoke_update_config(false);
        });
    }

    let audio_previewer = Arc::new(Mutex::new(AudioPreviewer::new()));
    let _audio_stopped_timer = {
        let main_window_weak = main_window.as_weak();
        let audio_previewer = audio_previewer.clone();
        audio_stopped_timer(audio_previewer, move || {
            let main_window_weak = main_window_weak.clone();
            slint::invoke_from_event_loop(move || {
                main_window_weak.unwrap().invoke_audio_stopped();
            }).unwrap();
        })
    };

    let (rt_handle, rt_tx) = {
        let main_window_weak = main_window.as_weak();
        render_thread::render_thread(move |msg| {
            match msg {
                RenderThreadMessage::Error(e) => {
                    let main_window_weak = main_window_weak.clone();

                    let mut info = default_progress_info(ProgressType::RenderError);
                    info.error = e.to_string().into();

                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_info(info);
                    }).unwrap();
                }
                RenderThreadMessage::RenderStarting => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(true);
                        main_window_weak.unwrap().set_progress_info(default_progress_info(ProgressType::RenderStarting));
                    }).unwrap();
                }
                RenderThreadMessage::RenderProgress(p) => {
                    let mut info = default_progress_info(ProgressType::Rendering);
                    info.progress = p.expected_duration_frames.map(|exp_dur_frames| p.frame as f32 / exp_dur_frames as f32).unwrap_or(0.0);
                    info.fps = p.average_fps as i32;
                    info.encoded_duration = slint_duration(p.encoded_duration);
                    info.expected_duration = slint_optional_duration(p.expected_duration);
                    info.video_size = p.encoded_size as i32;
                    info.eta = slint_optional_duration(p.eta_duration.map(|eta| eta.saturating_sub(p.elapsed_duration)));

                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_progress_info(info);
                    }).unwrap();
                }
                RenderThreadMessage::RenderComplete => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_info(default_progress_info(ProgressType::RenderFinished));
                    }).unwrap();
                }
                RenderThreadMessage::RenderCancelled => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_info(default_progress_info(ProgressType::RenderCancelled));
                    }).unwrap();
                }
            }
        })
    };
    
    let (spt_handle, spt_tx) = {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();

        sample_processing_thread::sample_processing_thread(move |msg| {
            match msg {
                SampleProcessingThreadMessage::Error(e) => {
                    let main_window_weak = main_window_weak.clone();
                    let mut info = default_progress_info(ProgressType::ProcessorError);
                    info.error = e.to_string().into();

                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_info(info);
                    }).unwrap();
                }
                SampleProcessingThreadMessage::ProcessingStarting => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_processing_samples(true);
                        main_window_weak.unwrap().set_progress_info(default_progress_info(ProgressType::ProcessorStarting));
                    }).unwrap();
                }
                SampleProcessingThreadMessage::ProcessingProgress(p) => {
                    let mut info = default_progress_info(ProgressType::Processing);

                    match p {
                        SampleProcessorProgress::DetectingSamples { current_frame, total_frames, detected_samples } => {
                            info.progress = (current_frame as f32 / total_frames as f32) / 2.0;
                            info.current_sample = 0;
                            info.total_samples = detected_samples as i32;
                        },
                        SampleProcessorProgress::ProcessingSamples { current_sample, total_samples, source } => {
                            info.progress = 0.5 + (current_sample as f32 / total_samples as f32) / 2.0;
                            info.source = source as i32;
                            info.current_sample = (current_sample + 1) as i32;
                            info.total_samples = total_samples as i32;
                        },
                        _ => unreachable!()
                    }

                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_progress_info(info);
                    }).unwrap();
                },
                SampleProcessingThreadMessage::ProcessingComplete(sample_data) => {
                    options.lock().unwrap().sample_tunings = sample_data;

                    let main_window_weak = main_window_weak.clone();
                    let options = options.clone();
                    slint::invoke_from_event_loop(move || {
                        let sample_configs: Vec<SampleConfig> = options
                            .lock()
                            .unwrap()
                            .sample_tunings
                            .iter()
                            .map(|(source, data)| SampleConfig {
                                name: "".into(),
                                source: *source as i32,
                                pitch_type: PitchType::Automatic,
                                base_frequency: data.base_pitch() as f32,
                                frequency: data.base_pitch() as f32,
                                amk_tuning: 3,
                                amk_subtuning: 0,
                                color: random_slint_color(),
                                use_color: false
                            })
                            .collect();

                        main_window_weak.unwrap().set_sample_configs(slint::ModelRc::new(slint::VecModel::from(sample_configs)));
                        main_window_weak.unwrap().set_processing_samples(false);
                        main_window_weak.unwrap().set_progress_info(default_progress_info(ProgressType::ProcessorFinished));
                    }).unwrap();
                },
                SampleProcessingThreadMessage::ProcessingCancelled => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_processing_samples(false);
                        main_window_weak.unwrap().set_progress_info(default_progress_info(ProgressType::ProcessorCancelled));
                    }).unwrap();
                }
            }
        })
    };

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        let spt_tx = spt_tx.clone();
        let localization_adapter = localization_adapter.clone();
        main_window.on_browse_for_module(move || {
            match browse_for_module_dialog() {
                Some(path) => {
                    let metadata_lines = match get_spc_metadata(&path) {
                        Ok((_duration, metadata_lines)) => metadata_lines,
                        Err(e) => {
                            let message = localization_adapter
                                .lock()
                                .unwrap()
                                .get("error-message-spc-file-invalid", Some(&fluent_args!(error: e.to_string())));
                            display_error_dialog(&message);
                            return options.lock().unwrap().input_path.clone().into();
                        }
                    };

                    options.lock().unwrap().input_path = path.clone();
                    main_window_weak.unwrap().set_metadata_lines(metadata_lines);

                    main_window_weak.unwrap().invoke_reformat_duration();

                    main_window_weak.unwrap().invoke_reset_sample_configs();
                    main_window_weak.unwrap().set_sample_configs(slint::ModelRc::new(slint::VecModel::from(vec![])));
                    spt_tx.send(SampleProcessingThreadRequest::CancelProcessing).unwrap();
                    spt_tx.send(SampleProcessingThreadRequest::StartProcessing(path.clone())).unwrap();

                    path.into()
                },
                None => options.lock().unwrap().input_path.clone().into()
            }
        });
    }

    {
        let options = options.clone();
        main_window.on_browse_for_background(move || {
            match browse_for_background_dialog() {
                Some(path) => {
                    options.lock().unwrap().video_options.background_path = Some(path.clone().into());

                    path.into()
                },
                None => options.lock().unwrap().video_options.background_path.clone().unwrap_or("".to_string()).into()
            }
        });
    }

    {
        let options = options.clone();
        main_window.on_background_cleared(move || {
            options.lock().unwrap().video_options.background_path = None;
        });
    }

    {
        let options = options.clone();
        main_window.on_get_duration(move |stop_condition_type, stop_condition_num| {
            let duration = match stop_condition_type {
                StopConditionType::Frames => {
                    let seconds = (stop_condition_num as f64) / 60.0;
                    Duration::from_secs_f64(seconds)
                },
                StopConditionType::Time => {
                    Duration::from_secs(stop_condition_num as _)
                },
                StopConditionType::SpcDuration => {
                    match get_spc_metadata(options.lock().unwrap().input_path.clone()) {
                        Ok((Some(duration), _metadata_lines)) => duration,
                        _ => return ERROR_DURATION
                    }
                }
            };
            slint_duration(duration)
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let localization_adapter = localization_adapter.clone();
        main_window.on_import_tunings(move || {
            let tuning_data_path = match browse_for_tuning_data() {
                Some(path) => path,
                None => return
            };

            let mut sample_configs: Vec<SampleConfig> = main_window_weak.unwrap().get_sample_configs()
                .iter()
                .collect();

            if tuning_data_path.ends_with(".json") {
                let session_json = match fs::read_to_string(tuning_data_path) {
                    Ok(json) => json,
                    Err(e) => {
                        let message = localization_adapter
                            .lock()
                            .unwrap()
                            .get("error-message-tuning-read-error", Some(&fluent_args!(error: e.to_string())));
                        display_error_dialog(&message);
                        return;
                    }
                };

                let samples = match tuning::super_midi_pak_session::SuperMidiPakSession::from_json(&session_json).and_then(|session| session.samples()) {
                    Ok(samples) => samples,
                    Err(e) => {
                        let message = localization_adapter
                            .lock()
                            .unwrap()
                            .get("error-message-tuning-parse-error", Some(&fluent_args!(error: e.to_string())));
                        display_error_dialog(&message);
                        return;
                    }
                };

                for sample in samples {
                    if let Some(config) = sample_configs.iter_mut().find(|config| config.source == sample.source as i32) {
                        if let Some(pitch) = sample.pitch {
                            config.frequency = pitch as f32;
                            config.pitch_type = PitchType::Frequency;
                        }
                        config.name = sample.name.clone().into();
                    }
                }
            } else {
                let message = localization_adapter
                    .lock()
                    .unwrap()
                    .get("error-message-tuning-unrecognized-data-format", None);
                display_error_dialog(&message);
                return;
            }

            main_window_weak.unwrap().set_sample_configs(slint::ModelRc::new(slint::VecModel::from(sample_configs)));
        });
    }

    {
        let options = options.clone();
        let audio_previewer = audio_previewer.clone();
        main_window.on_play_audio(move |sample_config| {
            let options = options.lock().unwrap();
            let mut audio_previewer = audio_previewer.lock().unwrap();
            let source = sample_config.source as u8;
            if let Some(sample_data) = options.sample_tunings.get(&source) {
                audio_previewer.play(sample_data.sample())
            } else {
                false
            }
        });
    }

    {
        let audio_previewer = audio_previewer.clone();
        main_window.on_stop_audio(move || {
            let mut audio_previewer = audio_previewer.lock().unwrap();
            audio_previewer.stop();
        });
    }

    {
        let audio_previewer = audio_previewer.clone();
        main_window.on_change_audio_pitch(move |sample_config, new_pitch| {
            let mut audio_previewer = audio_previewer.lock().unwrap();
            if new_pitch > 0 {
                audio_previewer.set_pitch(new_pitch as u16) as i32
            } else {
                let f_0 = match sample_config.pitch_type {
                    PitchType::Frequency => sample_config.frequency as f64,
                    PitchType::AddMusicK => 32000.0 / (16.0 * ((sample_config.amk_tuning as f64) + (sample_config.amk_subtuning as f64 / 256.0))),
                    PitchType::Automatic => sample_config.base_frequency as f64
                };
                audio_previewer.set_pitch_to_midi_note(f_0, new_pitch.abs()) as i32
            }
        });
    }

    {
        let options = options.clone();
        let localization_adapter = localization_adapter.clone();
        main_window.on_dump_sample(move |sample_config| {
            let options = options.lock().unwrap();
            let source = sample_config.source as u8;
            if let Some(sample_data) = options.sample_tunings.get(&source) {
                let output_path = match browse_for_dump_dialog() {
                    Some(path) => path,
                    None => return
                };

                let brr_data = sample_data.sample().to_bytes();
                if let Err(e) = fs::write(output_path, brr_data) {
                    let message = localization_adapter
                        .lock()
                        .unwrap()
                        .get("error-message-tuning-sample-write-error", Some(&fluent_args!(error: e.to_string())));
                    display_error_dialog(&message);
                }
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        let rt_tx = rt_tx.clone();
        main_window.on_start_render(move || {
            let output_path = match browse_for_video_dialog() {
                Some(path) => path,
                None => return
            };

            if output_path.ends_with(".mov") && confirm_prores_export_dialog() {
                // -c:v prores_ks -profile:v 4 -bits_per_mb 1000 -pix_fmt yuva444p10le
                options.lock().unwrap().video_options.video_codec = "prores_ks".to_string();
                options.lock().unwrap().video_options.video_codec_params.insert("profile".to_string(), "4".to_string());
                options.lock().unwrap().video_options.video_codec_params.insert("bits_per_mb".to_string(), "1000".to_string());
                options.lock().unwrap().video_options.pixel_format_out = "yuva444p10le".to_string();
            }

            options.lock().unwrap().video_options.output_path = output_path;
            options.lock().unwrap().fadeout_length = main_window_weak.unwrap().get_fadeout_duration() as u64;
            options.lock().unwrap().video_options.resolution_out.0 = main_window_weak.unwrap().get_output_width() as u32;
            options.lock().unwrap().video_options.resolution_out.1 = main_window_weak.unwrap().get_output_height() as u32;

            let stop_condition_num = main_window_weak.unwrap().get_stop_condition_num() as u64;
            options.lock().unwrap().stop_condition = match main_window_weak.unwrap().get_stop_condition_type() {
                StopConditionType::Frames => StopCondition::Frames(stop_condition_num),
                StopConditionType::Time => StopCondition::Frames(stop_condition_num * 60),
                StopConditionType::SpcDuration => StopCondition::SpcDuration
            };

            options.lock().unwrap().per_sample_colors.clear();

            let sample_configs: Vec<SampleConfig> = main_window_weak.unwrap().get_sample_configs()
                .as_any()
                .downcast_ref::<slint::VecModel<SampleConfig>>()
                .unwrap()
                .iter()
                .collect();
            for config in sample_configs {
                let source = config.source as u8;
                let custom_tuning = match config.pitch_type {
                    PitchType::Frequency => Some(config.frequency as f64),
                    PitchType::AddMusicK => Some(32000.0 / (16.0 * ((config.amk_tuning as f64) + (config.amk_subtuning as f64 / 256.0)))),
                    PitchType::Automatic => None
                };
                options.lock().unwrap().sample_tunings.get_mut(&source).unwrap().set_custom_tuning(custom_tuning);

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

                        Color::from_rgba8(r, g, b, 0xFF)
                    };
                    options.lock().unwrap().per_sample_colors.insert(config.source as u8, color);
                }
            }

            if options.lock().unwrap().video_options.background_path.clone().unwrap_or("".to_string()).is_empty() {
                options.lock().unwrap().video_options.background_path = None;
            }

            main_window_weak.unwrap().invoke_update_config(true);

            rt_tx.send(RenderThreadRequest::StartRender(options.lock().unwrap().clone())).unwrap();
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
    if spt_tx.send(SampleProcessingThreadRequest::Terminate).is_ok() {
        spt_handle.join().unwrap();
    }
}
