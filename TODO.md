# DX7 FM Synth — Pendiente

Motor FM, LFO y MIDI completos al nivel DX7/DX7S — ver `CHANGELOG.md`.

## Política

Ceñirse al **DX7 / DX7S**, saltarse las features exclusivas del **reface DX**.
Cada item lleva su origen entre paréntesis: `(DX7)`, `(DX7S)`, `(reface DX)`,
`(genérico)` o `(implementación)`.

> Heredado: la cadena `Chorus → Delay → Reverb` (`effects.rs`) precede a esta
> política. Se mantiene como utilidad genérica.

---

## Presets y persistencia

Loader JSON cubre todo el banco `mark/` (formato itsjoesullivan/dx7-patches).

- [ ] (genérico) Cargar archivo JSON individual — botón "Load JSON" o CLI
- [ ] (genérico) Cargar banco completo desde directorio
- [ ] (genérico) Guardar preset a archivo (JSON propio o SysEx)
- [ ] (genérico) Banco de usuario — slots editables además de los 32 ROM
- [ ] (genérico) Voice naming desde GUI
- [ ] (genérico) A/B comparison — portable desde `synth-analog-rs`

---

## GUI

**Controles que faltan exponer (motor ya los soporta):**

- [ ] (DX7) Key scaling con 4 curvas — dropdown −EXP/−LIN/+LIN/+EXP por lado,
      slider de profundidad por lado, selector de breakpoint
- [ ] (DX7) Transpose + pitch bend range por voz en panel VOICE

**Visualización (portar de `synth-analog-rs`):**

- [ ] (genérico) Osciloscopio + spectrum analyzer (`visualiser.rs` + `ScopeRing`)
- [ ] (genérico) VU meter dB-scaled con peak-hold
- [ ] (genérico) Curva ADSR en tiempo real
- [ ] (genérico) Highlight de operadores activos en el diagrama

**Widgets y browser (portar de `synth-analog-rs`):**

- [ ] (genérico) Knob circular (drag vertical, Shift+drag fino, doble click reset)
- [ ] (genérico) LED push buttons
- [ ] (genérico) Preset browser con búsqueda, categorías, A/B integrado, random

**Otros:**

- [ ] (implementación) Modularizar `gui.rs` (2300+ líneas) — ideal junto al
      upgrade de `eframe`/`egui`
- [ ] (genérico) Undo / Redo

---

## Calidad de audio

- [ ] (genérico) Soft clipper de salida (`tanh(x)`) — equivalente moderno al
      companding μ-law del D/A 12-bit del DX7
- [ ] (genérico) DC offset removal — high-pass primer orden (fc ~5–10 Hz)

---

## Rendimiento

- [ ] (implementación) SIMD para las 16 voces — solo si el CPU se vuelve
      bottleneck con polifonía máxima

---

## Deuda técnica

### Consolidar routing por controlador en `ControllerRoute` (implementación)

Mod Wheel / Aftertouch / Breath / Foot duplican campos en `SynthEngine`,
`SynthSnapshot`, `SynthCommand` (14+5 variantes) y `SynthController` (18
métodos). Refactor: 2 variantes parametrizadas
(`SetControllerSens { ctrl, dest, sens }`, `SetControllerValue`) y 4 structs
`ControllerRoute`. Toca 5 archivos + 4 tests.

Atacar cuando se añada un nuevo controlador o destino. Mientras tanto, el
coste supera al beneficio.

### `PMS_TABLE` para PITCH externo (DX7S — autenticidad)

PMS del patch usa tabla DX7 ROM exponencial; PITCH de AT/Breath/Foot usa
escala lineal vía `route_amount`. La etiqueta "7" del slider representa dos
depths distintos según destino.

Decidir antes de prometer "fidelidad DX7S" en docs públicas. Mientras se
posicione como "DX7-inspired", lineal es defendible. Documentar en
`docs/authenticity_policy.md`.

### Política única de clamp (implementación)

`Aftertouch / Breath / Foot / Expression` clampean en el motor; `ModWheel` y
`PitchBend` no. `midi_handler.rs` ya divide por 127.0 — los clamps son
redundantes e inconsistentes. Atacar junto al refactor de `ControllerRoute`.

---

## Dependencias

`rand`, `midir`, `cpal` actualizadas. MSRV: Rust 1.80.

- [ ] (implementación) **`eframe`/`egui` 0.28 → 0.34** — 6 minors de breaking
      changes. Branch dedicado, combinar con la modularización de `gui.rs`.

> **Validación auditiva pendiente** del upgrade reciente: probar con
> teclado real (notas, pitch bend, MIDI CC, SysEx) y stream de audio en
> CoreAudio antes de declararlo "funciona idénticamente".

---

## Omitido por política

- (reface DX) Efectos: Distortion, Flanger, Phaser, Touch Wah, 2 slots × 7 tipos
- (reface DX) Feedback por operador con tipo de onda (saw/square)
- (reface DX) Polyphonic Phrase Looper
