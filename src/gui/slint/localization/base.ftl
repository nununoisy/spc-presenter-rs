### Localization for SPCPresenter GUI strings
### Use this to create other translations.
### Please delete comments after translating!

## FileBrowser
# Default label for a FileBrowser.
file-browser-label-default = File:
# Text shown in the path textbox when no file is selected.
file-browser-path-placeholder = No file selected
# Button that opens the file selection dialog.
file-browser-button-browse = Browse...
# Button that deselects the currently selected file.
file-browser-button-clear = Clear

## Version dialog
# Button that opens the version dialog.
version-dialog-button-open = About
# Button that closes the version dialog.
version-dialog-button-close = Close

## SPC selection section
# Label for the FileBrowser that selects an SPC file.
spc-file-browser-label = Input SPC:
# Placeholder text shown when the current SPC file does not have metadata.
spc-metadata-placeholder = <no metadata>

## Configuration section
# Label for channel color pickers.
config-label-base-channel-colors = Base channel colors:
# Button that imports a TOML config file.
config-button-import-tooltip = Import
# Button that exports a TOML config file.
config-button-export-tooltip = Export
# Button that resets the current config to the default.
config-button-reset-tooltip = Reset

## Tuning section
# Label for the tuning section.
tuning-section-header = Sample tuning
# Button that imports tuning data from music production data files.
tuning-button-import = Import tunings

# Pitch type: automatic pitch detection
tuning-pitch-type-automatic = Automatic
# Pitch type: manually specified fundamental frequency
tuning-pitch-type-frequency = Frequency
# Pitch type: AddMusicK/Kankichi-kun tuning data
tuning-pitch-type-addmusick = AddMusicK tuning

## Tuning section: Sample configuration pane
# Placeholder name used for samples that do not have a name.
tuning-sample-name-placeholder = <no name>
# Placeholder text displayed in the config pane when no sample is selected.
tuning-sample-config-pane-placeholder = <no sample selected>
# Sample source index (VxSRCN) and name (from imported tuning data).
# $sourceIndex (String) - VxSRCN of the sample preformatted in hex.
# $sampleName (String) - Name of the sample from imported tuning data.
tuning-sample-config-sample-name = Sample {$sourceIndex}: {$sampleName}
# Button that dumps/extracts the selected sample to a BRR file.
tuning-sample-config-button-dump = Dump
# Button that decreases the octave of the preview piano keys.
tuning-sample-config-preview-button-decrease-octave-tooltip = Decrease preview octave
# Button that increases the octave of the preview piano keys.
tuning-sample-config-preview-button-increase-octave-tooltip = Increase preview octave
# Label for the combobox that selects the sample pitch type.
tuning-sample-config-pitch-type-label = Pitch:
# Unit suffix for Hertz.
tuning-sample-config-unit-suffix-hertz = Hz
# Placeholder text for the AddMusicK/Kankichi-kun tuning parameter. Keep this short!
tuning-sample-config-addmusick-tuning-placeholder = Tuning
# Placeholder text for the AddMusicK/Kankichi-kun subtuning parameter. Keep this short!
tuning-sample-config-addmusick-subtuning-placeholder = Sub.
# Label for the color picker that selects a specific color for the currently selected sample.
tuning-sample-config-per-sample-color-label = Per-sample color:

## Render options section
# Label for the FileBrowser that selects a background image/video for the render.
background-file-browser-label = Background:

# Label for the settings that determine the output video duration/play time.
render-duration-label = Render duration:
# Duration type: duration in seconds.
render-duration-type-seconds = seconds
# Duration type: duration in video frames.
render-duration-type-frames = frames
# Duration type: duration from the SPC Id666 header.
render-duration-type-spc = SPC duration

# Label for the numeric input that controls the length of the audio fade-out in video frames.
render-fadeout-duration-label = Fadeout frames:

# Label for the settings that determine the output video size/resolution.
render-output-resolution-label = Output video size:
# Preset video resolution: 1920x1080 (1080p)
render-output-resolution-preset-1080p = 1080p
# Preset video resolution: 3840x2160 (4K)
render-output-resolution-preset-4k = 4K

# Label for the combobox that selects the DSP sample interpolation type.
resampling-type-label = Sample interpolation:
# Sample interpolation type: Gaussian (accurate to the SNES).
resampling-type-accurate = Hardware-accurate Gaussian
# Sample interpolation type: Gaussian (higher quality than the SNES).
resampling-type-gaussian = High-fidelity Gaussian
# Sample interpolation type: Linear (worse than the SNES, but reminiscent of some old emulators).
resampling-type-linear = Linear

# Label for the checkbox that enables Blargg's SNES_DSP audio filter.
blargg-dsp-filter-checkbox-label = Use Blargg's DSP filter

# Button that starts rendering the video.
render-button-start = Render!
# Button that cancels the render.
render-button-cancel = Cancel

## Progress titles (text above progress bar)
# Progress title: currently idle.
progress-title-idle = Idle
# Progress title: initializing emulator before rendering/processing samples.
progress-title-starting = Setting up
# Progress title: rendering.
progress-title-rendering = Rendering
# Progress title: rendering, but the video length is not known yet because the loop point has not been encountered.
progress-title-rendering-to-loop-point = Rendering to loop point
# Progress title: initializing renderer.
progress-title-rendering-initializing = Initializing
# Progress title: detecting samples in the SPC.
progress-title-processing-detecting-samples = Detecting samples
# Progress title: processing the detected samples.
progress-title-processing-processing-samples = Processing samples

## Progress statuses (text below progress bar)
# Progress status: error while rendering.
# $error (String) - Error message.
progress-status-render-error = Render error: {$error}
# Progress status: initializing emulator before rendering.
progress-status-render-starting = Preparing your music
# Progress status: rendering.
# $progress (Number) - Progress percentage (0-100).
# $fps (Number) - Rendering speed in video frames per second.
# $encodedDuration (String) - How long the encoded video is currently, preformatted.
# $expectedDuration (String) - How long the encoded video will be when finished, preformatted.
# $videoSize (String) - Size of the encoded video on disk, preformatted.
# $eta (String) - How long the render will take from now until completion, preformatted.
progress-status-rendering = {$progress}%, {$fps} FPS, encoded {$encodedDuration}/{$expectedDuration} ({$videoSize}), {$eta}
# Progress status: rendering finished.
progress-status-render-finished = Render finished
# Progress status: rendering cancelled.
progress-status-render-cancelled = Render cancelled
# Progress status: error while processing samples.
# $error (String) - Error message.
progress-status-processor-error = Processing error: {$error}
# Progress status: initializing emulator before detecting samples for processing.
progress-status-processor-starting = Preparing to detect samples
# Progress status: detecting samples.
# $progress (Number) - Progress percentage (0-100).
# $detectedSamples (Number) - The number of samples that have been found so far.
progress-status-processing-detecting-samples = {$progress}%, found {$detectedSamples ->
    [one] {$detectedSamples} sample
    *[other] {$detectedSamples} samples
}
# Progress status: processing samples.
# $progress (Number) - Progress percentage (0-100).
# $sourceIndex (String) - VxSRCN of the sample preformatted in hex.
# $currentSample (Number) - The number of samples processed so far.
# $totalSamples (Number) - The number of samples that were detected.
progress-status-processing-processing-samples = {$progress}%, processing sample {$sourceIndex} ({$currentSample}/{$totalSamples})
# Progress status: processing finished.
progress-status-processor-finished = Processing finished
# Progress status: processing cancelled.
progress-status-processor-cancelled = Processing cancelled

## Formatted duration
# Formatted duration: error placeholder
formatted-duration-error = <error>
# Formatted duration: unknown duration placeholder
formatted-duration-unknown = <unknown>
# Number less than 10 that is part of a formatted duration
formatted-duration-component-lt10 = 0{$n}
# Number greater than or equal to 10 that is part of a formatted duration
formatted-duration-component-ge10 = {$n}
# Formatted duration (like how you would see on a stopwatch/chronometer).
formatted-duration = {$hours}:{$minutes}:{$seconds}

## Remaining duration
# Remaining duration: unknown duration placeholder
remaining-duration-unknown = unknown time remaining
# Remaining duration: $n hours left
# $n (Number) - hours
remaining-duration-hours = { $n ->
    [one] {$n} hour remaining
    *[other] {$n} hours remaining
}
# Remaining duration: $n minutes left
# $n (Number) - minutes
remaining-duration-minutes = { $n ->
    [one] {$n} minute remaining
    *[other] {$n} minutes remaining
}
# Remaining duration: $n seconds left
# $n (Number) - seconds
remaining-duration-seconds = { $n ->
    [one] {$n} second remaining
    *[other] {$n} seconds remaining
}
# Remaining duration: less than a second left
remaining-duration-less-than-a-second = less than a second remaining

## File sizes
# File size: $n gibibytes (2^30 bytes)
# $n (String) - gibibytes, preformatted
file-size-gibibytes = {$n} GiB
# File size: $n mebibytes (2^20 bytes)
# $n (String) - mebibytes, preformatted
file-size-mebibytes = {$n} MiB
# File size: $n kibibytes (2^10 bytes)
# $n (String) - kibibytes, preformatted
file-size-kibibytes = {$n} KiB
# File size: $n bytes
# $n (String) - bytes, preformatted
file-size-bytes = {$n} B
