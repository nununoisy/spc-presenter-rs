file-browser-label-default = File:
file-browser-path-placeholder = No file selected
file-browser-button-browse = Browse...
file-browser-button-clear = Clear

version-dialog-button-open = About
version-dialog-button-close = Close

spc-file-browser-label = Input SPC:
spc-metadata-placeholder = <no metadata>

config-label-base-channel-colors = Base channel colours:
config-button-import-tooltip = Import
config-button-export-tooltip = Export
config-button-reset-tooltip = Reset

tuning-section-header = Sample tuning
tuning-button-import = Import tunings

tuning-pitch-type-automatic = Automatic
tuning-pitch-type-frequency = Frequency
tuning-pitch-type-addmusick = AddMusicK tuning

tuning-sample-name-placeholder = <no name>
tuning-sample-config-pane-placeholder = <no sample selected>
tuning-sample-config-sample-name = Sample {$sourceIndex}: {$sampleName}
tuning-sample-config-button-dump = Dump
tuning-sample-config-preview-button-decrease-octave-tooltip = Decrease preview octave
tuning-sample-config-preview-button-increase-octave-tooltip = Increase preview octave
tuning-sample-config-pitch-type-label = Pitch:
tuning-sample-config-unit-suffix-octave = { $n ->
    [one] octave
    *[other] octaves
}
tuning-sample-config-unit-suffix-hertz = Hz
tuning-sample-config-addmusick-tuning-placeholder = Tuning
tuning-sample-config-addmusick-subtuning-placeholder = Sub.
tuning-sample-config-per-sample-color-label = Per-sample colour:

background-file-browser-label = Background:

render-duration-label = Render duration:
render-duration-type-seconds = seconds
render-duration-type-frames = frames
render-duration-type-spc = SPC duration

render-fadeout-duration-label = Fadeout frames:

render-output-resolution-label = Output video size:
render-output-resolution-preset-1080p = 1080p
render-output-resolution-preset-4k = 4K

resampling-type-label = Sample interpolation:
resampling-type-accurate = Hardware-accurate Gaussian
resampling-type-gaussian = High-fidelity Gaussian
resampling-type-linear = Linear
resampling-type-cubic = Cubic spline
resampling-type-sinc = Sinc

blargg-dsp-filter-checkbox-label = Use Blargg's DSP filter
dim-background-checkbox-label = Dim background

render-button-start = Render!
render-button-cancel = Cancel

progress-title-idle = Idle
progress-title-starting = Setting up
progress-title-rendering = Rendering
progress-title-rendering-to-loop-point = Rendering to loop point
progress-title-rendering-initializing = Initialising
progress-title-processing-detecting-samples = Detecting samples
progress-title-processing-processing-samples = Processing samples

progress-status-render-error = Render error: {$error}
progress-status-render-starting = Preparing your music
progress-status-rendering = { NUMBER($progress, style: "percent") }, {$fps} FPS, encoded {$encodedDuration}/{$expectedDuration} ({$videoSize}), {$eta}
progress-status-render-finished = Render finished
progress-status-render-cancelled = Render cancelled
progress-status-processor-error = Processing error: {$error}
progress-status-processor-starting = Preparing to detect samples
progress-status-processing-detecting-samples = { NUMBER($progress, style: "percent") }, found {$detectedSamples ->
    [one] {$detectedSamples} sample
    *[other] {$detectedSamples} samples
}
progress-status-processing-processing-samples = { NUMBER($progress, style: "percent") }, processing sample {$sourceIndex} ({$currentSample}/{$totalSamples})
progress-status-processor-finished = Processing finished
progress-status-processor-cancelled = Processing cancelled

error-message-spc-file-invalid = Invalid SPC file: {$error}
error-message-config-read-error = Failed to read configuration file: {$error}
error-message-config-parse-error = Invalid configuration file: {$error}
error-message-config-write-error = Failed to write configuration file: {$error}
error-message-config-serialize-error = Failed to serialise configuration: {$error}
error-message-tuning-read-error = Failed to read tuning data: {$error}
error-message-tuning-parse-error = Invalid tuning data: {$error}
error-message-tuning-unrecognized-format = Unrecognised tuning data format.
error-message-tuning-sample-write-error = Failed to dump sample: {$error}

file-dialog-filter-all = All supported formats
file-dialog-filter-config = TOML configuration file
file-dialog-filter-spc = SPC file
file-dialog-filter-brr = BRR sample
file-dialog-filter-video-background = Video background formats
file-dialog-filter-image-background = Image background formats
file-dialog-filter-super-midi-pak-session = Super MIDI Pak session
file-dialog-filter-output-mpeg4 = MPEG-4 video
file-dialog-filter-output-matroska = Matroska video
file-dialog-filter-output-quicktime = QuickTime video

prores-export-dialog-prompt = You have chosen to export a QuickTime video. Do you want to export in ProRes 4444 format to preserve alpha information for video editing? Note that ProRes 4444 is a lossless codec, so the exported file may be very large.

formatted-duration-error = <error>
formatted-duration-unknown = <unknown>
formatted-duration = { NUMBER($hours, minimumIntegerDigits: 2) }:{ NUMBER($minutes, minimumIntegerDigits: 2) }:{ NUMBER($seconds, minimumIntegerDigits: 2) }

remaining-duration-unknown = unknown time remaining
remaining-duration-hours = { $n ->
    [one] {$n} hour remaining
    *[other] {$n} hours remaining
}
remaining-duration-minutes = { $n ->
    [one] {$n} minute remaining
    *[other] {$n} minutes remaining
}
remaining-duration-seconds = { $n ->
    [one] {$n} second remaining
    *[other] {$n} seconds remaining
}
remaining-duration-less-than-a-second = less than a second remaining

file-size-gibibytes = { NUMBER($n, maximumFractionDigits: 2) } GiB
file-size-mebibytes = { NUMBER($n, maximumFractionDigits: 2) } MiB
file-size-kibibytes = { NUMBER($n, maximumFractionDigits: 2) } KiB
file-size-bytes = {$n} B
