# DX7 FM Synth — Pendiente

Verificado contra el código fuente actual. Solo items genuinamente ausentes.
Fuentes: DX7S manual (español), reface DX manual (ZT92120), colección de patches
[itsjoesullivan/dx7-patches](https://github.com/itsjoesullivan/dx7-patches) (mark/),
y repo hermano `synth-analog-rs` (features portables).

---

## 1. Motor FM ✅

Sección completada. Todo el motor FM está al nivel del DX7S y absorbe los patches
del banco `mark/` con todos sus campos. Detalles pendientes solo son refinamientos
que se moverán a otras secciones (GUI, presets, etc.).

### Operadores

- [x] **Fixed frequency mode** — `Operator::fixed_frequency` + `fixed_freq_hz`. La GUI
      expone toggle RATIO/FIXED y un slider logarítmico 1–4000 Hz. El JSON loader lee
      `oscillatorMode: "fixed"` y combina `fixedFrequencyCoarse` (0–3) y
      `fixedFrequencyFine` (0–99) según la fórmula DX7 `f = 10^coarse · (1 + fine/100)`.

- [x] **Coarse + Fine frequency** — JSON ya convertía `frequency: 0` → `0.5×`. Coarse
      como entero se preserva tal cual; el modo FIXED ahora usa coarse y fine para
      reconstruir Hz absolutos.

- [x] **Key scaling: 4 curvas + profundidad por lado** — `KeyScaleCurve` enum
      (`-LIN`, `-EXP`, `+EXP`, `+LIN`) + `key_scale_left/right_curve` y
      `key_scale_left/right_depth` por operador. `calculate_key_level_factor()`
      aplica curva lineal o exponencial al lado correspondiente. JSON
      `keyboardLevelScaling.{breakpoint, leftCurve, rightCurve, leftDepth, rightDepth}`
      se carga directamente. Breakpoints aceptan tanto `"A-1"` como enteros MIDI.

- [x] **AMS por operador** — `Operator::am_sensitivity` (0–3). Aplicado dentro de
      `process_inner()` con la tabla DX7 `[0%, 9%, 37%, 100%]`. La voz inyecta el
      LFO amp en cada op vía `set_lfo_amp_mod()` antes de procesar. JSON
      `amSensitivity` cargado.

- [x] **PMS por voz** — `SynthEngine::pitch_mod_sensitivity` (0–7) con tabla DX7
      `[0, 0.082, 0.16, 0.32, 0.5, 0.79, 1.26, 2.0]`. Multiplica el LFO pitch antes
      de pasarlo a las voces. JSON `lfo.pitchModSensitivity` cargado.

- [x] **Oscilador key sync desactivable** — `Operator::oscillator_key_sync`. Cuando
      es `false`, `trigger()` no resetea la fase. JSON `oscillatorKeySync` (`"On"`/`"Off"`)
      cargado a nivel de patch (todos los operadores comparten el flag, fiel al DX7).

### Pitch EG

- [x] **Pitch EG** — Nuevo módulo `pitch_eg.rs`: `PitchEg` struct con 4 rates + 4
      levels (50 = neutral, ±48 semitonos). Disparado por note-on, liberado al cerrar
      la voz. Sumado al pitch de cada voz vía `pitch_eg_semitones`. JSON `pitchEG`
      (clave en mayúsculas) cargado y activado automáticamente cuando algún level ≠ 50.

### Portamento / Afinación

- [x] **Mono-Legato** — `VoiceMode::MonoLegato`. Solo aplica portamento cuando la
      nota anterior sigue pulsada. `mono_held_order` (lista FIFO de notas pulsadas)
      permite que `note_off` retome la nota anterior y suprime re-disparo de
      LFO/Pitch EG en transiciones legato.

- [x] **Glissando** — `SynthEngine::portamento_glissando`. Cuantiza la frecuencia
      portamento al semitono más cercano vía `quantize_to_semitone()` por sample.

- [x] **Transpose** — `SynthEngine::transpose_semitones` (±24). Aplicado en
      `apply_transpose()` antes de generar la frecuencia. JSON `transpose: "C3"`
      → 0; `"C2"` → -12; integer directo. Persistido en `Dx7Preset::transpose_semitones`.

- [x] **Pitch bend range por preset** — `Dx7Preset::pitch_bend_range: Option<f32>`.
      `apply_to_synth()` invoca `synth.set_pitch_bend_range()` solo cuando está
      definido en el preset, conservando el global como fallback.

---

## 2. LFO ✅

- [x] **AMS / PMS** — Implementados en sección 1 (motor FM). PMS también expuesto en
      el panel LFO (slider 0–7 bajo "MOD WHEEL ROUTING").

- [x] **EG Bias** — Mod Wheel routing trio implementado:
      - **EG Bias (amp)**: `eg_bias_sensitivity` (0–7) en `SynthEngine`. La voz aplica
        `eg_bias_amount = mod_wheel · sens/7` a cada operador via `set_eg_bias`.
        Dentro de `process_inner` el factor `1 - eg_bias · ams_scale · 0.7` atenúa el
        operador, manteniendo la convención DX7 de que AMS=0 lo deja intacto y AMS=3
        recibe la atenuación máxima (~70%).
      - **Pitch Bias**: `pitch_bias_sensitivity` (0–7). Suma `mod_wheel · sens/7 · 2`
        semitonos al pitch total junto al PMS y la Pitch EG.
      - GUI: tres sliders (PMS / EG Bias / P-Bias) en el panel LFO, sección
        "MOD WHEEL ROUTING".
      - Routing por Foot/Breath/Aftertouch queda para sección 4 (MIDI), donde se
        añadirán los handlers para CC2/CC11/0xD0 y se reusará la misma infraestructura.

---

## 3. Efectos

Los cuatro efectos faltantes son del reface DX (7 tipos por slot). La arquitectura
actual es una cadena fija Chorus → Delay → Reverb.

- [ ] **Distortion** — Saturación no lineal (`tanh` o curva de transferencia). Parámetros:
      DRIVE, TONE (filtro post-distorsión). Útil también como soft-clipper de salida.

- [ ] **Flanger** — Como el chorus pero con retardos más cortos (1–5ms) y feedback
      alto. Produce comb filtering modulado. Parámetros: DEPTH, RATE.

- [ ] **Phaser** — All-pass stages en cascada moduladas por LFO. Parámetros: DEPTH, RATE.

- [ ] **Touch Wah** — Filtro paso-banda con resonancia, controlado por envelope follower
      de la amplitud de entrada. Parámetros: SENS (sensibilidad al nivel de entrada),
      REZ (resonancia del filtro). Feature exclusivo del reface DX.

- [ ] **2 slots de efectos configurables** — El reface DX tiene 2 slots en serie donde
      cada uno puede ser cualquiera de los 7 tipos. La arquitectura actual es fija.
      Refactorizar `EffectsChain` a `[Option<Box<dyn Effect>>; 2]` o similar.

- [ ] **Efectos por preset** — Los parámetros de efectos actuales son globales al
      sintetizador. Deberían guardarse y cargarse como parte del preset.

---

## 4. MIDI

- [ ] **Aftertouch (0xD0)** — Canal de presión monofónico. El DX7S define routing
      configurable a: PITCH sensitivity (0–7), AMPLITUDE (0–7), EG BIAS (0–7),
      PITCH BIAS (0–7). El `midi_handler.rs` no maneja el status byte 0xD0.

- [ ] **Breath Controller (CC2)** — El DX7S Function mode define BREATH CTRL PITCH,
      AMPLITUDE, EG BIAS, PITCH BIAS (0–7 cada uno). No manejado en `midi_handler.rs`.

- [ ] **Foot Controller** — Parámetros DX7S: FOOT CTRL VOLUME (0–15), PITCH (0–7),
      AMPLITUDE (0–7), EG BIAS (0–7). No implementado.

- [ ] **Expression (CC11)** — Controlador de expresión independiente del volumen
      principal. No manejado en `midi_handler.rs`.

- [ ] **Bank Select (CC0 / CC32)** — Selección de banco de presets combinada con
      Program Change para acceder a más de 128 presets.

- [ ] **SysEx recepción** — Formato DX7 estándar: 32 voces = 4104 bytes (F0 43 00 09
      20 00 ... F7), voz única = 163 bytes. Permite importar patches desde hardware
      DX7 real o cualquier editor SysEx.

- [ ] **SysEx envío** — Exportar la voz activa en formato SysEx DX7 para compatibilidad
      con editores externos y hardware real.

- [ ] **MIDI channel configurable** — Actualmente el handler acepta todos los canales
      (OMNI implícito). El DX7S permite configurar el canal de recepción (1–16 u OMNI).

---

## 5. Presets y persistencia

### Loader JSON (colección itsjoesullivan/dx7-patches)

Formato: un fichero JSON por voz. La colección `mark/` tiene 26 patches de alta
calidad. Estado actual del soporte:

| Campo JSON | Estado | Nota |
|---|---|---|
| name, algorithm, feedback | OK | |
| operators[].eg, outputLevel, detune, frequency | OK | frequency=0 → 0.5 corregido |
| lfo (wave/speed/delay/depths/sync) | OK | `amDepth` admite string o int |
| operators[].keyVelocitySensitivity | OK | mapeado a `velocity_sensitivity` (0–7) |
| operators[].keyboardRateScaling | OK | mapeado a `key_scale_rate` (0–7) |
| operators[].keyboardLevelScaling (curvas/profundidades) | OK | breakpoint admite `"A-1"` o entero MIDI |
| transpose | OK | `"C3"` → 0; `"C2"` → -12; entero directo |
| pitchEG | OK | clave literal `pitchEG` (mayúsculas) |
| lfo.pitchModSensitivity (PMS) | OK | tabla DX7 [0, .082, .16, .32, .5, .79, 1.26, 2.0] |
| operators[].amSensitivity | OK | tabla DX7 [0%, 9%, 37%, 100%] aplicada |
| operators[].oscillatorMode "fixed" | OK | usa `fixedFrequencyCoarse`/`Fine` para Hz |
| oscillatorKeySync (`On`/`Off`) | OK | fiel al DX7 (flag a nivel de voz) |

- [ ] **Cargar archivo JSON individual** — Botón "Load JSON" en GUI o argumento CLI.

- [ ] **Cargar banco completo desde directorio** — Cargar todos los `.json` de una
      carpeta como banco de presets navegable. La colección `mark/` tiene 26 voces.

### Persistencia general

- [ ] **Guardar preset a archivo** — Exportar la voz editada (JSON propio o SysEx).

- [ ] **Banco de usuario** — Slots editables además de los 32 ROM hardcoded.

- [ ] **Voice naming** — Editar el nombre del preset desde la GUI.

- [ ] **A/B comparison** — Guardar el estado antes de editar para alternar entre
      original y modificado. Implementado en `synth-analog-rs/src/gui/preset_browser.rs`,
      portable directamente.

---

## 6. GUI

### Controles faltantes

- [ ] **Panel Pitch EG** — Cuando se implemente el PEG, necesita 8 sliders (4 rates
      + 4 levels) con el mismo estilo visual que el EG de amplitud.

- [ ] **AMS / PMS por operador** — Controles en el panel de cada operador.

- [ ] **Fixed frequency toggle** — Toggle RATIO/FIXED y campo de frecuencia absoluta.

- [ ] **Key scaling con 4 curvas** — Dropdown para −EXP/−LIN/+LIN/+EXP por lado
      (izquierda y derecha), más sliders de profundidad independientes.

- [ ] **Transpose + pitch bend range por voz** — Controles globales de voz en el
      panel VOICE.

- [ ] **Selector Mono/Mono-Legato/Poly** — Tres modos en lugar del toggle actual
      Poly/Mono.

### Visualización (portar de `synth-analog-rs`)

- [ ] **Osciloscope + spectrum analyzer** — Implementación completa disponible en
      `synth-analog-rs/src/gui/visualiser.rs`. Incluye:
      - Osciloscopio con trigger estable (histéresis), zoom slider, auto-gain
      - FFT 2048-point con Hann window, suavizado exponencial por bin, marcadores de
        frecuencia (50Hz–20kHz), grid de dB a −12/−24/−36/−48
      - Requiere añadir `ScopeRing` a `lock_free.rs` (ya existe en el analog repo)

- [ ] **VU meter dB-scaled con peak-hold** — Implementación en
      `synth-analog-rs/src/gui/mod.rs` (`peak_db`, `peak_hold_db`, `VU_FLOOR_DB = -48`).
      Mucho más útil que un medidor lineal 0–1.

### Widgets (portar de `synth-analog-rs`)

- [ ] **Knob circular** — Implementación completa en
      `synth-analog-rs/src/widgets/knob.rs`. Features:
      - Drag vertical proporcional al rango, Shift+drag para ajuste fino (÷10)
      - Doble click para reset al default
      - Tooltip con valor numérico al hover
      - Arc visual de 270° con color amber
      Sustituiría sliders horizontales en el panel de operador.

- [ ] **LED push buttons** — El analog repo reemplazó dropdowns y checkboxes por
      botones LED estilo hardware (commit "Replace dropdowns and checkboxes with LED
      push buttons"). Más auténtico para los botones de modo.

### Preset browser (portar de `synth-analog-rs`)

- [ ] **Preset browser con búsqueda y categorías** — Implementación en
      `synth-analog-rs/src/gui/preset_browser.rs`. Incluye:
      - Búsqueda por nombre (singleline TextEdit con filtro live)
      - Filtro por categoría (Bass/Lead/Pad/Strings/Brass/FX/Sequence/Other)
      - Lista scrollable con agrupación visual por categoría
      - Panel Create/Edit con nombre + categoría
      - A/B comparison integrado
      - Random patch generator (valores acotados a rangos musicales)

### Otros

- [ ] **Visualización gráfica de EG** — Dibujar la curva ADSR de cada operador en
      tiempo real (triángulo Attack→Decay→Sustain→Release) en lugar de solo sliders.

- [ ] **Highlighting de operadores activos** — En el diagrama de algoritmo, iluminar
      los operadores cuyo EG sigue activo al tocar una nota.

- [ ] **Modularizar gui.rs** — El archivo tiene 1751 líneas. El analog repo lo divide
      en `gui/mod.rs`, `gui/panels.rs`, `gui/keyboard.rs`, `gui/preset_browser.rs`,
      `gui/visualiser.rs`, `gui/midi_windows.rs`. Aplicar la misma estructura.

- [ ] **Undo / Redo** — Historial de cambios de parámetros para deshacer ediciones.

---

## 7. Características exclusivas reface DX

- [ ] **Feedback por operador con tipo de onda** — El reface DX permite que cada
      operador tenga su propio feedback con dos modos según la posición del slider:
      - Arriba del centro → sawtooth (la onda evoluciona de seno puro a saw completa)
      - Abajo del centro → square (la onda evoluciona de seno puro a square completa)
      - Centro exacto → nivel 0, sinusoidal pura
      Es la innovación técnica principal del reface vs. el DX7. Requiere un campo
      `feedback_mode: FeedbackMode` en `Operator` y modificar `process_inner()`.

- [ ] **Polyphonic Phrase Looper** — Hasta 2000 notas o 10 minutos grabados como
      datos MIDI internos, con sobregrabación de capas. No afecta el motor de audio,
      solo re-emite eventos MIDI grabados.

---

## 8. Calidad de audio

- [ ] **Soft clipper de salida** — `tanh(x)` o curva personalizada antes de la salida
      de audio. Previene clipping duro cuando múltiples voces suman amplitudes altas.
      Una distortion suave implementada también cubre este caso.

- [ ] **DC offset removal** — Filtro high-pass de primer orden (fc ~5–10Hz) en la
      salida. El feedback puede acumular componente continua.

---

## 9. Rendimiento

- [ ] **SIMD para voces** — Las 16 voces son independientes y candidatas ideales para
      vectorización con `std::simd` (nightly) o `packed_simd`. Solo relevante si el
      CPU se convierte en bottleneck con polyphony máxima + efectos.

---

## Referencia rápida — lo que SÍ está implementado

Para no duplicar esfuerzo. Verificado en el código fuente.

| Feature | Archivo |
|---|---|
| 6 operadores, frequency_ratio, detune, output_level, feedback | `operator.rs` |
| EG 4-stage Rate/Level, exponential approach, key scale rate | `envelope.rs` |
| 32 algoritmos DX7 hardcoded (verificados vs Dexed/MSFA) | `algorithms.rs` |
| Feedback self (2-sample avg × PI/7), cross-feedback algs 4,6 | `operator.rs` |
| MOD_INDEX_SCALE = 4π (auténtico DX7) | `operator.rs:219` |
| Velocity sensitivity por operador (0–7, power curve) | `operator.rs` |
| Key scale level + rate + breakpoint (lineal simple) | `operator.rs` |
| 16 voces polifónicas, voice stealing (más antigua), fade in/out | `fm_synth.rs` |
| Mono mode con portamento (full) | `fm_synth.rs` |
| Sustain pedal (CC64) | `fm_synth.rs` |
| Master tune ±150 cents | `fm_synth.rs` |
| Pitch bend con rango configurable global (0–12 st) | `fm_synth.rs` |
| Portamento exponencial 5ms–2.5s | `fm_synth.rs:146` |
| LFO: 6 formas de onda DX7, rate 0–99, delay 0–99, key sync | `lfo.rs` |
| LFO: pitch_depth, amp_depth, mod_wheel scaling | `lfo.rs:208` |
| Chorus estéreo (LFO 90°, interpolación lineal) | `effects.rs` |
| Delay ping-pong estéreo | `effects.rs` |
| Reverb Schroeder (4 comb + 2 allpass por canal) | `effects.rs` |
| MIDI: Note On/Off, Pitch Bend, CC1 Mod Wheel, CC64 Sustain, CC123 Panic | `midi_handler.rs` |
| MIDI Program Change (0xC0) → carga preset 0–31 | `midi_handler.rs` |
| 32 presets ROM1A hardcoded | `presets.rs` |
| Lock-free: SPSC ringbuffer (GUI→Audio) + triple buffer (Audio→GUI) | `command_queue.rs`, `state_snapshot.rs` |
| Lookup tables: sin, exp, DX7 level→amp, voice scaling, MIDI freq | `optimization.rs` |
| Diagrama de algoritmo con layout automático | `gui.rs` |
| GUI modos: VOICE, OPERATOR, LFO, EFFECTS | `gui.rs` |
| Teclado virtual por computadora (Z–M, Q–U, octavas ↑↓) | `gui.rs` |
