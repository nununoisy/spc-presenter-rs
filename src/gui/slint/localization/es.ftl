file-browser-label-default = Archivo:
file-browser-path-placeholder = Ningún archivo seleccionado
file-browser-button-browse = Seleccionar...
file-browser-button-clear = Quitar

version-dialog-button-open = Acerca de
version-dialog-button-close = Cerrar

spc-file-browser-label = Archivo SPC:
spc-metadata-placeholder = <ningún metadatos>

config-label-base-channel-colors = Colores de canales:
config-button-import-tooltip = Importar
config-button-export-tooltip = Exportar
config-button-reset-tooltip = Reiniciar

tuning-section-header = Afinación de samples
tuning-button-import = Importar afinaciones

tuning-pitch-type-automatic = Automático
tuning-pitch-type-frequency = Frequencia
tuning-pitch-type-addmusick = Afinación AddMusicK

tuning-sample-name-placeholder = <ningún nombre>
tuning-sample-config-pane-placeholder = <ningún sample selectado>
tuning-sample-config-sample-name = Sample {$sourceIndex}: {$sampleName}
tuning-sample-config-button-dump = Copiar
tuning-sample-config-preview-button-decrease-octave-tooltip = Disminuir octava de avance
tuning-sample-config-preview-button-increase-octave-tooltip = Aumentar octava de avance
tuning-sample-config-pitch-type-label = Tono:
tuning-sample-config-unit-suffix-hertz = Hz
tuning-sample-config-addmusick-tuning-placeholder = Afin.
tuning-sample-config-addmusick-subtuning-placeholder = Sub.
tuning-sample-config-per-sample-color-label = Color para este sample:

background-file-browser-label = Fondo:

render-duration-label = Duración de video:
render-duration-type-seconds = segundos
render-duration-type-frames = fotogramas
render-duration-type-spc = duración de SPC

render-fadeout-duration-label = Duración en fotogramas de fundido:

render-output-resolution-label = Resolución de video:
render-output-resolution-preset-1080p = 1080p
render-output-resolution-preset-4k = 4K

resampling-type-label = Interpolación de samples:
resampling-type-accurate = Gaussian preciso al hardware
resampling-type-gaussian = Gaussian de alta fidelidad
resampling-type-linear = Lineal

blargg-dsp-filter-checkbox-label = Usar el filtro DSP de Blargg

render-button-start = ¡Renderizar!
render-button-cancel = Cancelar

progress-title-idle = Ocioso
progress-title-starting = Configurando
progress-title-rendering = Renderizando
progress-title-rendering-to-loop-point = Renderizando al punto de bucle
progress-title-rendering-initializing = Inicializando
progress-title-processing-detecting-samples = Detectando samples
progress-title-processing-processing-samples = Procesando samples

progress-status-render-error = Error mientras renderizaba: {$error}
progress-status-render-starting = Preparando su música
progress-status-rendering = { NUMBER($progress, style: "percent") }, {$fps} FPS, codificó {$encodedDuration}/{$expectedDuration} ({$videoSize}), {$eta}
progress-status-render-finished = Render finalizado
progress-status-render-cancelled = Render cancelado
progress-status-processor-error = Error mientras procesaba: {$error}
progress-status-processor-starting = Preparando para detectar samples
progress-status-processing-detecting-samples = { NUMBER($progress, style: "percent") }, detectó {$detectedSamples ->
    [one] {$detectedSamples} sample
    *[other] {$detectedSamples} samples
}
progress-status-processing-processing-samples = { NUMBER($progress, style: "percent") }, procesando sample {$sourceIndex} ({$currentSample}/{$totalSamples})
progress-status-processor-finished = Procesando finalizado
progress-status-processor-cancelled = Procesando cancelado

error-message-spc-file-invalid = Archivo SPC inválido: {$error}
error-message-config-read-error = Error al leer el archivo de configuración: {$error}
error-message-config-parse-error = Archivo configuración inválido: {$error}
error-message-config-write-error = Error al transcribir el archivo de configuración: {$error}
error-message-config-serialize-error = Error al serializar la configuración: {$error}
error-message-tuning-read-error = Error al leer los datos de afinación: {$error}
error-message-tuning-parse-error = Datos de afinación inválidos: {$error}
error-message-tuning-unrecognized-format = Datos de afinación tienen un formato desconocido.
error-message-tuning-sample-write-error = Error al copiar el sample: {$error}

formatted-duration-error = <error>
formatted-duration-unknown = <desconocido>
formatted-duration = { NUMBER($hours, minimumIntegerDigits: 2) }:{ NUMBER($minutes, minimumIntegerDigits: 2) }:{ NUMBER($seconds, minimumIntegerDigits: 2) }

remaining-duration-unknown = queda tiempo desconocido
remaining-duration-hours = { $n ->
    [one] queda {$n} hora
    *[other] quedan {$n} horas
}
remaining-duration-minutes = { $n ->
    [one] queda {$n} minuto
    *[other] quedan {$n} minutos
}
remaining-duration-seconds = { $n ->
    [one] queda {$n} segundo
    *[other] queda {$n} segundos
}
remaining-duration-less-than-a-second = queda menos de un segundo

file-size-gibibytes = { NUMBER($n, maximumFractionDigits: 2) } GiB
file-size-mebibytes = { NUMBER($n, maximumFractionDigits: 2) } MiB
file-size-kibibytes = { NUMBER($n, maximumFractionDigits: 2) } KiB
file-size-bytes = {$n} B
