# DX7 FM Synth — Pendiente

Verificado contra el código fuente actual. Solo items genuinamente ausentes.
Fuentes: DX7S manual (español), reface DX manual (ZT92120), colección de patches
[itsjoesullivan/dx7-patches](https://github.com/itsjoesullivan/dx7-patches) (mark/),
y repo hermano `synth-analog-rs` (features portables).

## Política de autenticidad

Cada item está etiquetado con el origen de la feature. Política actual del proyecto:
**ceñirse al DX7 / DX7S y saltarse las features exclusivas del reface DX**.

| Etiqueta | Significado |
|---|---|
| **DX7** | Feature del DX7 original (1983) — implementar |
| **DX7S** | Añadida en DX7II / DX7S (1986–87, mismo motor + mejoras de control) — implementar |
| **reface DX** | Solo del reface DX (2015) — saltar (a menos que haya razón explícita) |
| **genérico** | Ni DX7 ni reface DX, sino utilidad práctica de cualquier sintetizador moderno (audio quality, GUI, persistencia) — evaluar caso a caso |
| **implementación** | Detalle interno (rendimiento, modularización) — sin origen específico |

> **Nota histórica:** Ni el DX7 ni el DX7S incluían **efectos** internos. Los
> Chorus/Delay/Reverb actualmente en el código son una herencia de inspiración
> reface DX que ya estaba antes de aplicar esta política.

---

## 1. Motor FM ✅ *(todo DX7/DX7S)*

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

## 2. LFO ✅ *(todo DX7/DX7S)*

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

## 3. Efectos — *sección omitida bajo la política DX7/DX7S*

Ni el DX7 ni el DX7S llevan efectos internos. Todos los items de esta sección
provienen del reface DX. Se documentan aquí como referencia pero **no se
implementan** mientras la política sea ceñirse al DX7/DX7S.

> **Estado actual:** la cadena fija `Chorus → Delay → Reverb` ya estaba en el
> código antes de aplicar esta política. Se mantiene como utilidad pero se
> considera un legado **genérico** (no DX7-auténtico). Si se quisiera pureza
> total habría que retirar `effects.rs` o exponerlo solo como FX externo
> opcional vía MIDI Out.

- [ ] *(reface DX)* **Distortion** — `tanh` con DRIVE + TONE post-LP. Útil
      también como soft-clipper, función que sí cabe en sección 8.

- [ ] *(reface DX)* **Flanger** — Retardos cortos (1–5 ms) con feedback alto.

- [ ] *(reface DX)* **Phaser** — Cascada de all-pass moduladas.

- [ ] *(reface DX)* **Touch Wah** — Band-pass + envelope follower. Único reface DX.

- [ ] *(reface DX)* **2 slots configurables** — La arquitectura del reface DX
      con 2 slots × 7 tipos. Implica refactor invasivo de `EffectsChain`.

- [ ] *(reface DX / genérico)* **Efectos por preset** — Persistencia de los
      parámetros del FX en el JSON del preset. Aplica solo si decides mantener
      la cadena de efectos heredada.

---

## 4. MIDI

Toda esta sección es **DX7 / DX7S nativa** — los controladores y el formato SysEx
son exactamente los del DX7 original. La infraestructura de routing por sensibilidad
(0–7) ya está parcialmente lista en sección 1+2 (PMS, EG Bias, Pitch Bias del Mod
Wheel) y se reusa para Foot/Breath/Aftertouch.

- [ ] *(DX7S)* **Aftertouch (0xD0)** — Canal de presión monofónico. Routing
      configurable a: PITCH sensitivity (0–7), AMPLITUDE (0–7), EG BIAS (0–7),
      PITCH BIAS (0–7). El `midi_handler.rs` no maneja el status byte 0xD0.

- [ ] *(DX7)* **Breath Controller (CC2)** — Function mode: BREATH CTRL PITCH,
      AMPLITUDE, EG BIAS, PITCH BIAS (0–7 cada uno).

- [ ] *(DX7)* **Foot Controller** — DX7S: FOOT CTRL VOLUME (0–15), PITCH (0–7),
      AMPLITUDE (0–7), EG BIAS (0–7).

- [ ] *(genérico)* **Expression (CC11)** — Controlador estándar MIDI. No es del
      DX7 original (es CC11) pero todos los teclados modernos lo envían.

- [ ] *(genérico)* **Bank Select (CC0 / CC32)** — MIDI estándar para acceder a
      más de 128 presets. El DX7 original solo tenía Program Change.

- [ ] *(DX7)* **SysEx recepción** — Formato DX7 estándar: 32 voces = 4104 bytes
      (F0 43 00 09 20 00 ... F7), voz única = 163 bytes.

- [ ] *(DX7)* **SysEx envío** — Exportar voz activa en formato SysEx DX7.

- [ ] *(DX7)* **MIDI channel configurable** — DX7S permite canal 1–16 u OMNI;
      hoy aceptamos todo (OMNI implícito).

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

- [ ] *(genérico)* **Cargar archivo JSON individual** — Botón "Load JSON" en
      GUI o argumento CLI.

- [ ] *(genérico)* **Cargar banco completo desde directorio** — Cargar todos
      los `.json` de una carpeta como banco navegable.

### Persistencia general

- [ ] *(genérico)* **Guardar preset a archivo** — Exportar voz editada (JSON
      propio o SysEx — este último sí es DX7).

- [ ] *(genérico)* **Banco de usuario** — Slots editables además de los 32 ROM.

- [ ] *(genérico)* **Voice naming** — Editar nombre del preset desde la GUI.

- [ ] *(genérico)* **A/B comparison** — Implementado ya en
      `synth-analog-rs/src/gui/preset_browser.rs`, portable.

---

## 6. GUI

### Controles faltantes

- [ ] *(DX7)* **Panel Pitch EG completo** — Hoy el motor sí calcula la PEG y se
      carga desde JSON, pero la GUI no expone los 8 sliders (4 rates + 4 levels).
      Necesita el mismo estilo visual que el EG de amplitud.

- [x] *(DX7)* ~~AMS por operador~~ — Slider 0–3 ya disponible en el panel de operador.

- [x] *(DX7)* ~~PMS por voz~~ — Slider 0–7 en el panel LFO bajo "MOD WHEEL ROUTING".

- [x] *(DX7)* ~~Fixed frequency toggle~~ — Toggle RATIO/FIXED + slider 1–4000 Hz
      en el panel de operador.

- [ ] *(DX7)* **Key scaling con 4 curvas (UI)** — El motor las soporta pero la GUI
      sigue exponiendo un único slider lineal "Key Lvl" que dirige ambos lados
      con la misma profundidad. Necesita: dropdown −EXP/−LIN/+LIN/+EXP por lado
      + slider de profundidad independiente + selector de breakpoint.

- [ ] *(DX7)* **Transpose + pitch bend range por voz** — Controles globales en
      el panel VOICE. Ambos campos ya existen en el motor y se cargan desde el
      preset, pero no tienen control GUI directo (solo se cambian al cargar voz).

- [x] *(DX7)* ~~Selector Mono/Mono-Legato/Poly~~ — Selector de tres modos disponible
      en la barra superior y en `draw_mode_controls_compact`.

### Visualización (portar de `synth-analog-rs`)

Todas estas son **genérico** — utilidades de cualquier sintetizador moderno, no
específicas del DX7.

- [ ] *(genérico)* **Osciloscope + spectrum analyzer** — Implementación
      completa en `synth-analog-rs/src/gui/visualiser.rs`. Requiere `ScopeRing`
      en `lock_free.rs`.

- [ ] *(genérico)* **VU meter dB-scaled con peak-hold** — Más útil que un
      medidor lineal 0–1.

### Widgets (portar de `synth-analog-rs`)

- [ ] *(genérico)* **Knob circular** — Drag vertical, Shift+drag fino, doble
      click reset, tooltip, arc 270° amber. Sustituiría sliders horizontales.

- [ ] *(genérico)* **LED push buttons** — Estilo hardware más auténtico para
      botones de modo.

### Preset browser (portar de `synth-analog-rs`)

- [ ] *(genérico)* **Preset browser con búsqueda y categorías** — Búsqueda por
      nombre, filtro por categoría, lista scrollable, A/B integrado, random
      patch generator.

### Otros

- [ ] *(genérico)* **Visualización gráfica de EG** — Curva ADSR en tiempo real.

- [ ] *(genérico)* **Highlighting de operadores activos** — Iluminar ops con
      EG vivo en el diagrama de algoritmo.

- [ ] *(implementación)* **Modularizar gui.rs** — `gui.rs` tiene 1800+ líneas.
      `synth-analog-rs` lo divide en `gui/{mod, panels, keyboard, preset_browser,
      visualiser, midi_windows}.rs`.

- [ ] *(genérico)* **Undo / Redo** — Historial de cambios de parámetros.

---

## 7. Características exclusivas reface DX — *omitida bajo la política DX7/DX7S*

Lista de referencia. Bajo la política actual no se implementa.

- [ ] *(reface DX)* **Feedback por operador con tipo de onda** — Cada operador
      con su propio feedback en dos modos: saw arriba / square abajo del centro.
      Innovación técnica principal del reface vs. DX7. Requiere `feedback_mode:
      FeedbackMode` en `Operator` y modificar `process_inner()`.

- [ ] *(reface DX)* **Polyphonic Phrase Looper** — Hasta 2000 notas o 10 minutos
      como datos MIDI internos. No afecta el motor de audio.

---

## 8. Calidad de audio

- [ ] *(genérico)* **Soft clipper de salida** — `tanh(x)` o curva custom antes
      de la salida. Previene clipping duro al sumar muchas voces. El DX7 usa
      conversión D/A 12-bit con companding (μ-law) que también suaviza picos —
      un soft clipper digital es el equivalente moderno apropiado.

- [ ] *(genérico)* **DC offset removal** — Filtro high-pass de primer orden
      (fc ~5–10 Hz) en la salida. El feedback puede acumular componente continua.

---

## 9. Rendimiento

- [ ] *(implementación)* **SIMD para voces** — Las 16 voces son candidatas
      ideales para vectorización con `std::simd` (nightly) o `packed_simd`.
      Solo relevante si el CPU se vuelve bottleneck con polyphony máxima.

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
