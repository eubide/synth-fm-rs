# DX7 FM Synth — Pendiente

Verificado contra el código fuente actual. Solo items genuinamente ausentes.
Fuentes: DX7S manual (español), reface DX manual (ZT92120), colección de patches
[itsjoesullivan/dx7-patches](https://github.com/itsjoesullivan/dx7-patches) (mark/),
y repo hermano `synth-analog-rs` (features portables).

---

## 1. Motor FM

### Operadores

- [ ] **Fixed frequency mode** — Cada operador debería poder operar en modo RATIO
      (el actual, `frequency_ratio` escalado por la nota MIDI) o FIXED (frecuencia
      absoluta en Hz independiente de la nota). El DX7S define: OSC MODE RATIO/FIXED,
      COARSE (0–31) y FINE (0–99). Esencial para percusión y campanas.

- [ ] **Coarse + Fine frequency** — El formato JSON estándar (y el SysEx DX7) usa
      COARSE entero (0–31) donde `0 → 0.5×`, `1–31 → n×`. Actualmente `frequency_ratio`
      es un float libre. Se necesita la conversión en el loader JSON:
      `if coarse == 0 { 0.5 } else { coarse as f32 }`. Sin esto, patches con coarse=0
      (funk-bass, rock-lead, sax-2) producen silencio.

- [ ] **Key scaling: 4 curvas + profundidad independiente por lado** — El DX7S tiene
      LEFT CURVE / RIGHT CURVE (cada uno: −LIN, −EXP, +EXP, +LIN) y LEFT DEPTH /
      RIGHT DEPTH (0–99) independientes del breakpoint. Actualmente solo existe
      `key_scale_level` (float lineal, sin distinción izquierda/derecha ni tipo de curva).

- [ ] **AMS por operador** — Amplitude Modulation Sensitivity (0–3). Escala cuánto
      afecta el LFO a la amplitud de cada operador individualmente. El `lfo_amp_mod`
      actual se aplica igual a todos los carriers. El sax-2 y my-bells de Mark usan
      AMS intenso en casi todos los operadores.

- [ ] **PMS por voz** — Pitch Modulation Sensitivity (0–7). Escala la profundidad de
      modulación de pitch del LFO para toda la voz. En la lógica actual, `pitch_depth`
      y `mod_wheel` determinan el pitch mod sin ningún factor PMS. Afecta vibrato en
      brasshorns, celo, strg-ens-2, rock-lead (colección mark/).

- [ ] **Oscilador key sync desactivable** — `operator.rs::trigger()` siempre hace
      `self.phase = 0.0`. El DX7 tiene OSC KEY SYNC (ON/OFF): OFF deja los osciladores
      correr libremente entre notas (crea fases distintas cada vez). El JSON tiene
      `oscillatorKeySync: "Off"` y actualmente lo ignoramos.

### Pitch EG

- [ ] **Pitch EG** — Envolvente de tono independiente con 4 rates (0–99) + 4 levels
      (0–99, donde 50 = tono estándar, <50 = más bajo, >50 = más alto, rango ±4
      octavas). Produce glides de inicio en brass (brasshorns, brtrumpet de mark/)
      y vibratos programados que evolucionan. No hay ningún struct ni campo relacionado
      en el código. Requiere: nuevo struct `PitchEG`, campo en `SynthEngine`, nuevo
      `SynthCommand::SetPitchEGParam`, panel GUI, carga desde JSON.

### Portamento / Afinación

- [ ] **Mono-Legato** — Portamento solo cuando la nota anterior sigue presionada
      (legato). Actualmente solo existe FULL (portamento en cualquier nota en mono mode).
      El DX7S Function mode tiene parámetro PORTAMENTO MODE: RETAIN / FOLLOW.

- [ ] **Glissando** — Portamento con pasos discretos de semitono en lugar de glide
      continuo. Parámetro PORTAMENTO STEP (ON/OFF) del DX7S Function mode.

- [ ] **Transpose** — Desplazamiento en semitonos (±24, C3 = 0) aplicado antes del
      pitch bend. Guardado por preset. No hay ningún campo en `SynthEngine`. Necesario
      para cargar celo y strg-ens-2 de la colección mark/ (ambos tienen `transpose: C2`).

- [ ] **Pitch bend range por preset** — Actualmente `pitch_bend_range` es un parámetro
      global del sintetizador. El DX7S define el rango (0–12 semitones) como parte de
      la voz. Necesario para carga fiel desde JSON/SysEx.

---

## 2. LFO

- [ ] **AMS / PMS** — Ver sección Operadores. Son el mismo feature.

- [ ] **EG Bias** — Fuente de modulación separada documentada en el diagrama de bloques
      del DX7S (pág. 26). Es un offset de pitch o amplitud controlable por Mod Wheel,
      Foot Controller o Breath Control, distinto del LFO: el LFO oscila, el EG Bias es
      un offset estático/controlable. Se aplica al mismo destino (pitch o amp) pero con
      un valor fijo en lugar de un valor oscilante.

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
| lfo (wave/speed/delay/depths/sync) | **ignorado** | `JsonPatch` no tiene campo `lfo`; todos los parches tienen datos LFO |
| operators[].keyVelocitySensitivity | **ignorado** | `JsonOperator` no lo deserializa; todos los valores 0–7 presentes en mark/ |
| operators[].keyboardRateScaling | **ignorado** | `JsonOperator` no lo deserializa; valores 0–7 en uso en mark/ |
| operators[].keyboardLevelScaling (curvas/profundidades) | **ignorado** | varios parches tienen leftDepth/rightDepth > 0 y curvas -EXP/+LIN |
| transpose | ignorado | celo, strg-ens-2 suenan una octava alta |
| pitchEG | ignorado | brasshorns, brtrumpet sin glide inicial |
| lfo.pitchModSensitivity (PMS) | ignorado | vibrato de mod wheel incorrecto |
| operators[].amSensitivity | ignorado | sax-2, my-bells sin tremolo correcto |
| operators[].oscillatorMode "fixed" | ignorado | ningún parche de mark/ lo usa → no urgente |

- [ ] **JSON loader: keyVelocitySensitivity por operador** — `JsonOperator` no
      deserializa `keyVelocitySensitivity`. Añadir el campo (0–7) y propagarlo al
      `Operator` al construir el preset. El motor de síntesis ya lo soporta
      (`operator.rs`). Valores no triviales en presets como epiano-1, sax-2,
      brtrumpet (impacto alto en expresividad).

- [ ] **JSON loader: keyboardRateScaling por operador** — `JsonOperator` no
      deserializa `keyboardRateScaling` (0–7). El motor ya tiene soporte parcial
      de key scale rate (`envelope.rs`). Mapear el valor al preset al cargarlo.

- [ ] **JSON loader: cargar LFO desde patch** — `JsonPatch` no tiene struct `lfo`.
      Añadir `JsonLfo { wave, speed, delay, pitch_mod_depth, am_depth, sync,
      pitch_mod_sensitivity }` y mapear a los campos del sintetizador. `amDepth`
      es string o int según el patch → requiere `#[serde(deserialize_with)]` custom.
      Afecta a todos los 25 parches de mark/ (todos tienen sección `lfo`).

- [ ] **JSON loader: amSensitivity por operador** — `JsonOperator` no deserializa
      `amSensitivity` (0–3). Depende de que el LFO esté cargado (item anterior).
      Valores no triviales en sax-2 y my-bells de mark/.

- [ ] **JSON loader: keyboardLevelScaling con curvas** — El campo actual solo
      usa un float lineal. El formato DX7 define breakpoint (nota) + leftCurve /
      rightCurve (−LIN, −EXP, +EXP, +LIN) + leftDepth / rightDepth (0–99).
      Varios parches de mark/ tienen profundidades no triviales (hasta 99) y curvas
      −EXP/+LIN que dan el balance correcto por registro de teclado (piano-3,
      brtrumpet, epiano-1).

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
