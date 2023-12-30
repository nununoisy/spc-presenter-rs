use anyhow::{Result, Context, bail};
use clap::{arg, ArgAction, value_parser, Command};
use std::path::PathBuf;
use indicatif::{FormattedDuration, HumanBytes, ProgressBar, ProgressStyle};
use std::fmt::Write;
use std::fs;
use std::str::FromStr;
use csscolorparser::Color as CssColor;
use tiny_skia::Color;
use crate::config::Config;
use crate::renderer::{Renderer, render_options::{RendererOptions, StopCondition}};
use crate::sample_processing::{SampleProcessor, SampleProcessorProgress};
use crate::tuning;

fn codec_option_value_parser(s: &str) -> Result<(String, String)> {
    let (key, value) = s.split_once('=')
        .context("Invalid option specification (must be of the form 'option=value').")?;

    Ok((key.to_string(), value.to_string()))
}

fn sample_tuning_numeric_parser(s: &str) -> Result<u8> {
    if s.starts_with('$') {
        u8::from_str_radix(&s[1..], 16)
            .with_context(|| format!("Invalid hex number {}", s))
    } else if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16)
            .with_context(|| format!("Invalid hex number {}", s))
    } else {
        u8::from_str(s)
            .with_context(|| format!("Invalid number {}", s))
    }
}

fn sample_tuning_value_parser<'a>(s: &'a str) -> Result<(u8, f64)> {
    let mut parts = s.split(':');

    let sample_index = sample_tuning_numeric_parser(parts.next().context("Invalid tuning specification (must be of the form 'sample_index:type:param,param,...').")?)?;
    let tuning_type: &'a str = parts.next().context("Invalid tuning specification (missing tuning type).")?;
    let raw_params: &'a str = parts.next().context("Invalid tuning specification (missing tuning parameters).")?;
    if parts.next().is_some() {
        bail!("Invalid tuning specification (too many options specified).");
    }

    let frequency = match tuning_type.to_ascii_lowercase().as_str() {
        "hz" => f64::from_str(raw_params)?,
        "amk" => {
            let (raw_tuning, raw_subtuning) = raw_params.split_once(',')
                .context("Invalid AddMusicK tuning parameters (must be of the form 'tuning,subtuning').")?;

            let tuning = sample_tuning_numeric_parser(raw_tuning)? as f64;
            let subtuning = sample_tuning_numeric_parser(raw_subtuning)? as f64;

            32000.0 / (16.0 * (tuning + subtuning / 256.0))
        }
        invalid => bail!("Invalid tuning type '{}' (must be one of 'hz', 'amk').", invalid)
    };

    Ok((sample_index, frequency))
}

fn sample_color_value_parser(s: &str) -> Result<(u8, Color)> {
    let (sample_index_str, color_str) = s.split_once(':')
        .with_context(|| format!("Invalid color specification '{}' (must be of the form 'source_index=color').", s))?;

    let sample_index = sample_tuning_numeric_parser(sample_index_str)?;
    let parsed_color = color_str.parse::<CssColor>()?;

    Ok((sample_index, Color::from_rgba(
        parsed_color.r as f32,
        parsed_color.g as f32,
        parsed_color.b as f32,
        parsed_color.a as f32
    ).unwrap()))
}

fn get_renderer_options() -> RendererOptions {
    let matches = Command::new("SPCPresenter")
        .arg(arg!(-c --"video-codec" <CODEC> "Set the output video codec")
            .required(false)
            .default_value("libx264"))
        .arg(arg!(-C --"audio-codec" <CODEC> "Set the output audio codec")
            .required(false)
            .default_value("aac"))
        .arg(arg!(-f --"pixel-format" <FORMAT> "Set the output video pixel format")
            .required(false)
            .default_value("yuv420p"))
        .arg(arg!(-F --"sample-format" <FORMAT> "Set the output audio sample format")
            .required(false)
            .default_value("fltp"))
        .arg(arg!(-R --"sample-rate" <RATE> "Set the output audio sample rate")
            .required(false)
            .value_parser(value_parser!(i32))
            .default_value("44100"))
        .arg(arg!(-s --"stop-at" <CONDITION> "Set the stop condition")
            .required(false)
            .value_parser(value_parser!(StopCondition))
            .default_value("time:300"))
        .arg(arg!(-S --"stop-fadeout" <FRAMES> "Set the audio fadeout length in frames")
            .required(false)
            .value_parser(value_parser!(u64))
            .default_value("180"))
        .arg(arg!(--"ow" <WIDTH> "Set the output video width")
            .required(false)
            .value_parser(value_parser!(u32))
            .default_value("1920"))
        .arg(arg!(--"oh" <HEIGHT> "Set the output video height")
            .required(false)
            .value_parser(value_parser!(u32))
            .default_value("1080"))
        .arg(arg!(-o --"video-option" <OPTION> "Pass an option to the video codec (option=value)")
            .required(false)
            .value_parser(codec_option_value_parser)
            .action(ArgAction::Append))
        .arg(arg!(-O --"audio-option" <OPTION> "Pass an option to the audio codec (option=value)")
            .required(false)
            .value_parser(codec_option_value_parser)
            .action(ArgAction::Append))
        .arg(arg!(-t --"manual-tune" <TUNING> "Manually specify sample tuning (sample_index:type:param,param,...)")
            .required(false)
            .value_parser(sample_tuning_value_parser)
            .action(ArgAction::Append))
        .arg(arg!(-P --"per-sample-color" <COLOR> "Specify per-sample color (sample_index:css_color)")
            .required(false)
            .value_parser(sample_color_value_parser)
            .action(ArgAction::Append))
        .arg(arg!(--"super-midi-pak-session" <SESSIONJSON> "Tune samples using a Super MIDI Pak session JSON file")
            .required(false)
            .value_parser(value_parser!(PathBuf)))
        .arg(arg!(-B --"background" <BACKGROUND> "Set the output background")
            .required(false)
            .value_parser(value_parser!(PathBuf)))
        .arg(arg!(-i --"import-config" <CONFIGFILE> "Import configuration from a RusticNES TOML file.")
            .value_parser(value_parser!(PathBuf))
            .required(false))
        .arg(arg!(<spc> "SPC to render")
            .value_parser(value_parser!(PathBuf))
            .required(true))
        .arg(arg!(<output> "Output video file")
            .value_parser(value_parser!(PathBuf))
            .required(true))
        .get_matches();

    let mut options = RendererOptions::default();

    options.input_path = matches.get_one::<PathBuf>("spc").cloned().unwrap().to_str().unwrap().to_string();
    options.video_options.output_path = matches.get_one::<PathBuf>("output").cloned().unwrap().to_str().unwrap().to_string();
    options.video_options.video_codec = matches.get_one::<String>("video-codec").cloned().unwrap();
    options.video_options.audio_codec = matches.get_one::<String>("audio-codec").cloned().unwrap();
    options.video_options.pixel_format_out = matches.get_one::<String>("pixel-format").cloned().unwrap();
    options.video_options.sample_format_out = matches.get_one::<String>("sample-format").cloned().unwrap();

    let sample_rate = matches.get_one::<i32>("sample-rate").cloned().unwrap();
    options.video_options.sample_rate = sample_rate;
    options.video_options.audio_time_base = (1, sample_rate).into();

    options.stop_condition = matches.get_one::<StopCondition>("stop-at").cloned().unwrap();
    options.fadeout_length = matches.get_one::<u64>("stop-fadeout").cloned().unwrap();

    let ow = matches.get_one::<u32>("ow").cloned().unwrap();
    let oh = matches.get_one::<u32>("oh").cloned().unwrap();
    options.video_options.resolution_out = (ow, oh);

    if let Some(video_options) = matches.get_many::<(String, String)>("video-option") {
        for (k, v) in video_options.cloned() {
            options.video_options.video_codec_params.insert(k, v);
        }
    }
    if let Some(audio_options) = matches.get_many::<(String, String)>("audio-option") {
        for (k, v) in audio_options.cloned() {
            options.video_options.audio_codec_params.insert(k, v);
        }
    }

    options.config = match matches.get_one::<PathBuf>("import-config") {
        Some(config_path) => {
            let config = fs::read_to_string(config_path).expect("Failed to read config file!");
            Config::from_toml(&config).expect("Failed to parse config file!")
        },
        None => Config::default()
    };

    let mut sample_processor = SampleProcessor::from_spc(options.input_path.clone()).expect("Failed to initialize sample processor");
    if let StopCondition::Frames(frames) = options.stop_condition {
        sample_processor.set_frame_count(frames as usize + options.fadeout_length as usize + 60);
    }
    loop {
        match sample_processor.step().unwrap() {
            SampleProcessorProgress::Finished => break,
            _ => ()
        }
    }
    options.sample_tunings = sample_processor.finish();

    if let Some(super_midi_pak_session_path) = matches.get_one::<PathBuf>("super-midi-pak-session").cloned() {
        let session_json = fs::read_to_string(super_midi_pak_session_path).unwrap();
        let session = tuning::super_midi_pak_session::SuperMidiPakSession::from_json(session_json.as_str()).unwrap();
        println!("Loaded Super MIDI Pak session version {}", session.version());
        for sample in session.samples().unwrap() {
            println!("Decoded sample: {}", &sample);
            if let Some(tuning) = options.sample_tunings.get_mut(&sample.source) {
                tuning.set_custom_tuning(sample.pitch);
            }
        }
    }
    if let Some(manual_tunings) = matches.get_many::<(u8, f64)>("manual-tune") {
        for (source, pitch) in manual_tunings.cloned() {
            if let Some(tuning) = options.sample_tunings.get_mut(&source) {
                tuning.set_custom_tuning(Some(pitch));
            }
        }
    }
    if let Some(sample_colors) = matches.get_many::<(u8, Color)>("per-sample-color") {
        for (sample_index, color) in sample_colors.cloned() {
            options.per_sample_colors.insert(sample_index, color);
        }
    }
    if let Some(background_path) = matches.get_one::<PathBuf>("background").cloned() {
        options.video_options.background_path = Some(background_path.to_str().unwrap().to_string());
    }

    options
}

pub fn run() {
    let options = get_renderer_options();
    let mut renderer = Renderer::new(options).unwrap();

    let pb = ProgressBar::new(0);
    let pb_style_initial = ProgressStyle::with_template("{msg}\n{spinner} Waiting for loop detection...")
        .unwrap();
    let pb_style = ProgressStyle::with_template("{msg}\n{wide_bar} {percent}%")
        .unwrap();
    pb.set_style(pb_style_initial);

    renderer.start_encoding().unwrap();
    loop {
        if !(renderer.step().unwrap()) {
            break;
        }

        if pb.length().unwrap() == 0 {
            if let Some(duration) = renderer.expected_duration_frames() {
                pb.set_length(duration as u64);
                pb.set_style(pb_style.clone());
            }
        }
        pb.set_position(renderer.current_frame());

        let current_video_duration = FormattedDuration(renderer.encoded_duration());
        let current_video_size = HumanBytes(renderer.encoded_size() as u64);
        let current_encode_rate = renderer.encode_rate();
        let expected_video_duration = match renderer.expected_duration() {
            Some(duration) => FormattedDuration(duration).to_string(),
            None => "?".to_string()
        };
        let elapsed_duration = FormattedDuration(renderer.elapsed()).to_string();
        let eta_duration = match renderer.eta_duration() {
            Some(duration) => FormattedDuration(duration).to_string(),
            None => "?".to_string()
        };

        let mut message: String = "VID]".to_string();
        write!(message, " enc_time={}/{}", current_video_duration, expected_video_duration).unwrap();
        write!(message, " size={}", current_video_size).unwrap();
        write!(message, " rate={:.2}", current_encode_rate).unwrap();

        write!(message, "\nEMU]").unwrap();
        write!(message, " pos=? loop={}", renderer.loop_count()).unwrap();
        write!(message, " fps={} avg_fps={}", renderer.instantaneous_fps(), renderer.average_fps()).unwrap();

        write!(message, "\nTIM]").unwrap();
        write!(message, " run_time={}/{}", elapsed_duration, eta_duration).unwrap();

        pb.set_message(message);
    }

    pb.finish_with_message("Finalizing encode...");
    renderer.finish_encoding().unwrap();
}