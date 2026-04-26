# DX7 FM Synth — Pendiente

Motor FM, LFO y MIDI completos al nivel DX7/DX7S — ver `CHANGELOG.md`.

## Política

Ceñirse al **DX7 / DX7S**, saltarse las features exclusivas del **reface DX**.
Cada item lleva su origen entre paréntesis: `(DX7)`, `(DX7S)`, `(reface DX)`,
`(genérico)` o `(implementación)`.

> Heredado: la cadena `Chorus → Delay → Reverb` (`effects.rs`) precede a esta
> política. Se mantiene como utilidad genérica.

---

## Comparativa con Dexed/MSFA

Análisis del motor frente a Dexed (motores `FmCore` "Modern" y `EngineMkI`
"Mark I") y MSFA. Cada item lleva el archivo de referencia en Dexed para que
el cambio sea verificable.

### Diferencias con impacto audible — candidatos a fijar

- [x] (DX7) **Tabla AMS ROM** — portado en `operator.rs:AMS_SCALE_TABLE`
      con valores ROM `{0, 66, 109, 255}/255`.

- [ ] (DX7) **Curva de envelope dB-lineal** — nuestro envelope avanza en
      dominio de amplitud (`envelope.rs:108-111`,
      `current_level += distance * approach_factor`). El DX7 / Dexed avanza
      en dominio dB y exponencializa al final vía `Exp2::lookup`
      (`env.cc:84-96`). El attack además salta a un punto ~40 dB sobre el
      suelo (jumptarget=1716). Diferencia perceptible: decays largos en
      escala lineal "se quedan colgados" más tiempo que en escala dB.
      Refactor grande — solo si la fidelidad lo justifica.

- [ ] (DX7) **LFO delay con fade-in** — el delay del DX7 son **dos fases**:
      espera plana + rampa lineal hasta full-depth. Nosotros sólo tenemos la
      espera plana: `lfo.rs:171-178` devuelve `(0, 0)` durante el delay y
      luego salta a depth completo. La modulación entra de golpe en vez del
      swell característico. Ref: `lfo.cc:105-117` (dos pendientes
      `delayinc_` y `delayinc2_`).

- [x] (DX7) **LFO range Hz** — portado en `lfo.rs:LFO_FREQ_TABLE` con la
      ROM `lfoSource[100]` (rate 99 ≈ 49 Hz como hardware real).

- [x] (DX7) **Velocity sensitivity ROM** — portado en
      `operator.rs:VELOCITY_DATA` + cálculo dB-step que replica
      `ScaleVelocity` de Dexed (`((sens * (data[v>>1] - 239) + 7) >> 3) << 4`).

- [x] (DX7) **Key scaling con `exp_scale_data[33]`** — portado en
      `operator.rs:EXP_SCALE_DATA` + nueva fórmula `ScaleLevel` /
      `ScaleCurve` con offset `breakpoint - 17` y agrupación de 3 semitonos.

### Diferencias menores — auditar antes de tocar

- [x] (DX7) **PMS table** — portado en `fm_synth.rs:PMS_TABLE` con la
      ROM `pitchmodsenstab[8] = {0, 10, 20, 33, 55, 92, 153, 255}` reescalada
      `× 2/255` para preservar ~2 semitonos de swing en PMS=7.

- [ ] (DX7) **Detune ±7 cents fijo vs nota-dependiente** — `operator.rs:268`
      hace `2^(detune/1200)`. Dexed: `0.0209 * exp(-0.396 * logfreq) / 7`
      (`dx7note.cc:61-62`), que da ~±3 cents en notas medias y mucho más
      en graves; medido por Pascal Gauthier en su DX7. Diferencia en
      milicents, prioridad baja.

- [ ] (DX7) **Voice scaling 1/√N vs 1/16 fijo** — `optimization.rs:73`
      atenúa por `sqrt` (RMS-preserving); Dexed siempre `>> 4` (1/16,
      peak-preserving) en `PluginProcessor.cpp:268-273`. El nuestro suena
      más fuerte con pocas voces y permite clipping más rápido en acordes
      densos; el de Dexed mantiene headroom constante. Decisión de carácter,
      no bug. Documentar en `authenticity_policy.md`.

- [ ] (DX7) **Voice stealing por prioridad multi-criterio** — robamos por
      "oldest first" (`fm_synth.rs`). Dexed prefiere por orden:
      no-playing → key-up + mismo pitch → key-up distinto → key-down + mismo
      pitch (re-trigger) → key-down distinto (steal real). Reduce clicks
      audibles. Ref `PluginProcessor.cpp:467-490` (`chooseNote`).

- [x] (DX7) **Pitch EG codificación lineal** — sustituido por las tablas
      ROM `PITCHENV_TAB[100]` y `PITCHENV_RATE[100]` con aproximación lineal
      en log-freq (estilo Dexed). El cálculo de semitonos usa
      `pitchenv_tab[level] * 0.375` y el incremento por sample es
      `pitchenv_rate[rate] * 12 / (21.3 * sr)`.

### Donde nuestro motor SUPERA a Dexed (no tocar)

- **Sin LUT 4096 + Catmull-Rom cúbica** vs Dexed 1024 + lineal
  (`sin.cc`, MSFA wiki SinePoly). 4× más resolución horizontal y mejor
  interpolación. Coste ~20 KB; beneficio: menos aliasing en armónicos altos.
- **Cross-feedback en algoritmos 4 y 6** — el motor `FmCore` "Modern" de
  Dexed NO implementa cross-feedback multi-op (comentario explícito
  `fm_core.cc:114`: "todo: more than one op in a feedback loop"). Sólo el
  motor `EngineMkI` lo hace bien. Nuestro `cross_feedback_signal()` +
  `process_no_self_feedback()` ya equivale a `compute_fb2`/`compute_fb3` de
  MkI: alg 4 encadena Op4→Op6→Op5→Op4, alg 6 encadena Op5→Op6→Op5.
- **DC blocker 5 Hz + tanh soft-clip post-mix** (`dc_blocker.rs`,
  `fm_synth.rs:soft_clip`). Dexed depende del clip duro a int16 + ladder
  Obxd opcional. El nuestro es más limpio en monitores de estudio.
- **Float32 internamente** vs Q24 fixed-point en Dexed/MSFA. Más rango
  dinámico, sin artefactos de truncación.

### Verificado contra Dexed (no requiere acción)

- **Self-feedback** `(last+prev)/2 * fb * PI/7` (`operator.rs:350-355`).
  Match con `(y0+y) >> (fb_shift+1)` donde fb=7 → shift=2 → `(y0+y)>>2 ≈
  ±π rad`. Coincide con la wiki Dx7Hardware.
- **32 algoritmos hardcoded** (`algorithms.rs`). Equivalente funcional a
  la tabla `algorithms[32]` byte-encoded de Dexed (`fm_core.cc:29-62`),
  más legible aunque menos compacto.
- **Operator level 99 = 0 dB, paso ≈ 0.75 dB** (`optimization.rs:79-90`).
  Match con `levellut + <<5` y "32 substeps × 0.0235 dB" de la wiki MSFA.
- **16 voces, mono/poly/legato** (`fm_synth.rs MAX_VOICES = 16`). Match.
- **Sample rate adaptable, sin oversampling** — mismo planteamiento que
  Dexed. Constantes y tablas reescaladas al SR del host.
- **Envelope rate 0 ≈ 38 s** (`optimization.rs:108`); Dexed mide ~40 s en
  TF1 real (`env.cc statics[0]=1764000/44100`). Dentro del 5% — los autores
  de MSFA admiten que su tabla "necesitaría doble verificación".
- **OSC KEY SYNC**, transpose, master tune, pitch bend range, sustain
  pedal: paridad con DX7 hardware y con Dexed.

### Notas

- Dexed tiene **dos motores DSP** intercambiables ("Modern/PD" y "Mark I").
  Nuestro motor está conceptualmente más cerca de Mark I por su tratamiento
  de cross-feedback. Si algún día se quisiera modo de compatibilidad estricta
  bit-exact, replicar el modelo Q24 fixed-point sería el camino — pero coste
  alto y beneficio acústico marginal sobre nuestra precisión float actual.
- Los autores de MSFA documentan que la tabla `statics[]` del envelope viene
  de medir 2 unidades de TF1 y "could probably use some double-checking and
  cleanup" (`env.cc`). No hay tabla ROM canónica pública del DX7.
- El detune ±7 fue derivado por Pascal Gauthier midiendo *un* DX7 propio:
  variabilidad unidad-a-unidad del hardware no está modelada en ningún
  emulador.

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
- [ ] (DX7) Curva del envelope DX7 (R1/L1…R4/L4) en tiempo real — el DX7
      usa 4 rates / 4 levels por operador, no ADSR clásico
- [x] (genérico) Highlight de operadores activos en el diagrama (brillo
      modulado por envelope live, max entre voces activas)

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

Cadena de salida: `tanh` soft clip → HPF 5 Hz por canal. Ver `CHANGELOG.md`.

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
