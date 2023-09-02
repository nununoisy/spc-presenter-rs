<p align="center">
    <img src="spc-presenter-icon-xl.png" alt="SPCPresenter logo" />
</p>

# SPCPresenter

SPCPresenter is a tool I wrote to generate visualizations of SNES/SFC
chiptunes, based on [snes-apu][snes-apu], [FFmpeg][ffmpeg],
and [Slint][slint].
The visualization design is essentially a port of the piano roll from
[RusticNES][rusticnes].

## Functionality

SPCPresenter runs your input SPC in a custom fork of snes-apu and
tracks the state of the emulated S-DSP chip every frame. It then
generates a visualization and feeds it to FFmpeg to be encoded as a
video.

## Features

- Supports SPC modules.
  - Support for songs that require 5A22/65816 emulation (SMC/SNSF) are planned.
- Based on a custom fork of snes-apu with a much more accurate S-DSP:
  - Reorders operations to more closely match a cycle-accurate implementation.
  - Properly emulates internal register sizes and wraps/clamps values accurately.
  - Implements all S-DSP register reads and writes, many of which were previously missing.
  - Reimplements the FIR filter to properly overflow and use the correct taps.
  - Correctly mixes negative channel volumes (SNES "surround").
  - Fixes a race condition that is triggered when dynamically resizing the echo buffer.
  - Remaining inaccuracies to be fixed:
    - BRR samples are decoded one block at a time instead of 4 samples at a time.
    - ENDX latch timing is slightly inaccurate which can lead to timing errors in
      some drivers.
    - S-DSP is not yet cycle-accurate. It instead runs in 64 SMP-clock periods.
- Automatic BRR sample analysis:
  - Rips samples from the S-DSP as the song plays, and tunes them using
    the YIN pitch detection algorithm.
    - Works quite well for monophonic pitched samples and some percussion.
  - Manual tuning parameters can be specified per source index:
    - As a fundamental frequency in Hz
    - As an AddMusicK tuning multiplier and submultiplier
    - From Super MIDI Pak session JSON files.
    - Support is planned for automatic imports from AddMusicK TXT files
      and XM/IT files.
  - Support is planned for automatic percussion classification.
  - Support is planned for automatic polyphonic sample pitch detection.
- Outputs a video file:
    - Customizable resolution (default 1080p) at 60 FPS.
    - MPEG-4 container with fast-start (`moov` atom at beginning of file).
    - Matroska (MKV) and QuickTime (MOV) containers are also supported.
    - yuv420p H.264 video stream encoded with libx264, crf: 16.
    - If using QuickTime, ProRes 4444 streams encoded with prores_ks are also supported.
    - Stereo AAC LC audio stream encoded with FFmpeg's aac encoder, bitrate: 384k.
- Video files are suitable for direct upload to most websites:
    - Outputs the recommended format for YouTube, Twitter, and Discord (w/ Nitro).
    - Typical H.264 exports (1080p, up to 3 minutes) are usually below 100MB.
- Video files have metadata based on SPC Id666 tags (title, artist, game name).
  - Support for extended Id666 is planned.

## Installation

**Windows**: head to the Releases page and grab the latest binary release. Simply unzip
and run the executable, and you're all set.

**Linux**: no binaries yet, but you can compile from source. You'll need to have `ffmpeg`
and optionally `Qt6` development packages installed, then clone the repo and run
`cargo build --release` to build.

## Usage

### GUI

1. Click **Browse...** to select an input module.
2. The module's metadata will be displayed.
3. Select the duration of the output video. Available duration types are:
    - Seconds: explicit duration in seconds.
    - Frames: explicit duration in frames (1/60 of a second).
    - SPC duration: if present, the duration specified in the Id666 tags of
      the SPC file.
4. Select the duration of the fadeout in frames. This is not included in the
   video duration above, rather it's added on to the end.
5. Select the output video resolution. You can enter a custom resolution
   or use the 1080p/4K presets.
6. Optionally select a background for the visualization. You can select many
   common image and video formats to use as a background.
    - You can also elect to export a transparent video later if you would like
      to use a video editor.
    - *Note:* Video backgrounds must be 60 FPS, or they will play at
      the wrong speed. A fix for this is planned.
7. Select additional rendering options:
    - Use Blargg's DSP post-filter: applies the band-pass filter from `snes_spc/SPC_Filter.cpp`.
    - Accurate interpolation: uses a more accurate Gaussian kernel for sample interpolation.
8. Click **Render!** to select the output video filename and begin rendering
   the visualization.
    - If you would like to render a transparent video for editing, then choose
      a filename ending in `.mov` to export in a QuickTime container. When asked
      if you would like to export using ProRes 4444, select **OK**.
9. Once the render is complete, you can select another track or even change
   modules to render another tune.

### CLI

If SPCPresenter is started with command line arguments, it runs in CLI mode.
This allows for the automation of rendering visualizations which in turn
allows for batch rendering and even automated uploads.

The most basic invocation is this:
```
spc-presenter-rs path/to/music.spc path/to/output.mp4
```

Additional options:
- `-R [rate]`: set the sample rate of the audio (default: 44100)
- `-s [condition]`: select the output duration (default: `time:300`):
    - `time:[seconds]`
    - `frames:[frames]`
    - `time:spc` (if supported)
- `-S [fadeout]`: select the fadeout duration in frames (default: 180).
- `--ow [width]`: select the output resolution width (default: 1920)
- `--oh [height]`: select the output resolution height (default: 1080)
- `-t [source_index]:[tuning_type]:[params]`: Specify manual tuning for a sample
  by source index:
    - `0:hz:500.0` tunes source index 0 to 500.0 Hz at pitch $1.000
    - `$12:amk:$03,0x32` tunes source index 18 like an AddMusicK instrument with a
      tuning multiplier of $03 and a submultiplier of $32.
    - You can use this flag multiple times to tune more than one sample.
- `--super-midi-pak-session [json_path]`: load tuning parameters from a Super MIDI
  Pak session JSON file.
- `-P [sample_index]:[css_color]`: Specify a color to be associated with a sample.
    - `-P 0:#ff9800` sets the color for source index 0 to `#ff9800`.
    - `-P $12:hsl(120, 100%, 63%)` sets the color for source index 18 to `#40ff40`.
    - Colors are parsed with the [`csscolorparser` crate][csscolorparser].
- `-B [background_file]`: add a background to the rendered video.
- `-h`: Additional help + options
    - Note: options not listed here are unstable and may cause crashes or
      other errors.

[snes-apu]: https://github.com/emu-rs/snes-apu
[rusticnes]: https://github.com/zeta0134/rusticnes-core
[ffmpeg]: https://github.com/FFmpeg/FFmpeg
[slint]: https://slint-ui.com
[csscolorparser]: https://docs.rs/csscolorparser/latest/csscolorparser/