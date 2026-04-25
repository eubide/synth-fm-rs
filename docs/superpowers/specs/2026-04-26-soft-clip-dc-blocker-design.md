# Soft Clipper (`tanh`) y DC Blocker

**Fecha:** 2026-04-26
**Origen:** `TODO.md` — Calidad de audio
**Política:** `(genérico)` — equivalente moderno al companding μ-law del DAC 12-bit del DX7

## Objetivo

1. Sustituir el soft-knee limiter actual (`soft_limit`, threshold 0.85 + knee 0.15) por
   `tanh(x)` — saturación suave simétrica, asintótica a ±1.0. Da el carácter
   cálido del DAC del DX7 antes del clip duro.
2. Añadir un HPF de primer orden (fc ≈ 5 Hz) al final de la cadena para eliminar
   DC offset residual.

## Contexto del código

Cadena de audio actual (`fm_synth.rs`):

```
voices → mix → master_volume → soft_limit  (mono, dentro de process())
       → effects.process()   (devuelve stereo)
       → soft_limit per-canal (dentro de process_stereo())
```

- `process()` mono: `fm_synth.rs:900` — termina en `soft_limit`.
- `process_stereo()`: `fm_synth.rs:985` — encadena efectos y aplica `soft_limit`
  per-canal otra vez.
- `soft_limit()`: `fm_synth.rs:1117` — soft-knee con cap 0.95.
- Único caller real: `audio_engine.rs:94` llama a `process_stereo()`. La mono
  `process()` no se invoca desde fuera.

## Diseño

### 1. Reemplazar `soft_limit` por `soft_clip` (tanh puro)

```rust
fn soft_clip(&self, sample: f32) -> f32 {
    sample.tanh()
}
```

- Se mantiene la firma (`&self`, `f32 → f32`) — sustitución directa.
- Se aplica en los **mismos dos sitios** que `soft_limit` actualmente:
  - Final de `process()` mono (defensivo antes de efectos).
  - Per-canal en `process_stereo()` después de los efectos.
- La doble aplicación es intencional: efectos como reverb/delay pueden
  amplificar por encima de 1.0, y el primer `soft_clip` evita que el reverb
  alimente picos extremos.
- Renombrar a `soft_clip` deja claro el cambio de naturaleza (saturación, no
  limiter por umbral).

### 2. `DcBlocker` — HPF primer orden

Filtro estándar:

```
y[n] = x[n] - x[n-1] + R · y[n-1]
```

con `R = 1 - 2π·fc/fs`. Para `fc = 5 Hz`, `fs = 44100 Hz`:
`R ≈ 0.999287`.

```rust
pub struct DcBlocker {
    prev_input: f32,
    prev_output: f32,
    r: f32,
}

impl DcBlocker {
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let r = 1.0 - 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        Self { prev_input: 0.0, prev_output: 0.0, r }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.prev_input + self.r * self.prev_output;
        self.prev_input = input;
        self.prev_output = output;
        output
    }
}
```

- Archivo nuevo: `src/dc_blocker.rs` (~30 líneas con tests).
- Coeficiente calculado en `new()` — permite cambiar `fc` o `sample_rate` en
  el futuro sin tocar la lógica.
- Stateful: dos instancias en `SynthEngine` (`dc_blocker_l`, `dc_blocker_r`).

### 3. Nueva cadena en `process_stereo()`

```rust
pub fn process_stereo(&mut self) -> (f32, f32) {
    let mono = self.process();                      // ya incluye soft_clip
    let (left, right) = self.effects.process(mono);
    let l = self.dc_blocker_l.process(self.soft_clip(left));
    let r = self.dc_blocker_r.process(self.soft_clip(right));
    (l, r)
}
```

DC blocker después del clip — el `tanh` puede introducir asimetría leve si la
señal ya tenía DC; aplicarlo después garantiza salida centrada en 0.

### 4. Cambios en `SynthEngine` struct

Añadir campos:

```rust
dc_blocker_l: DcBlocker,
dc_blocker_r: DcBlocker,
```

Inicializados en el constructor con `DcBlocker::new(sample_rate, 5.0)`.

## Tests

### `dc_blocker.rs`

- `process(0.0)` repetido → 0.0.
- DC puro (`x = 0.5`, 1 segundo de muestras) → output converge a ~0 (≤ 1e-3).
- AC a 1 kHz (sinusoide) → amplitud preservada (±5%).
- AC a 100 Hz → amplitud preservada (≥ 95%).

### `fm_synth.rs` — `soft_clip`

- `soft_clip(0.0) == 0.0`.
- `soft_clip(10.0)` ≈ 1.0 (tolerancia 1e-4).
- `soft_clip(-10.0)` ≈ -1.0.
- Monotónico: `soft_clip(0.5) < soft_clip(0.8) < soft_clip(2.0)`.

## Validación auditiva

No automatizable — requiere oído humano.

- Reproducir presets representativos (graves, brass, strings, bell) con MIDI real.
- Comparar volumen percibido pre/post — `tanh` empieza a comprimir desde ~0.5
  mientras que `soft_limit` no tocaba hasta 0.85. Posible pérdida de pegada
  percutiva.
- Si se siente aplastado: el usuario sube `master_volume` o se evalúa volver
  a aplicar `soft_clip` solo en `process_stereo` post-efectos (eliminando la
  llamada en `process()` mono).

## Archivos tocados

- `src/dc_blocker.rs` — nuevo (~30 líneas + tests).
- `src/fm_synth.rs` — añadir `mod dc_blocker` import, dos campos en struct,
  inicialización en constructor, modificar `process_stereo()`, reemplazar
  cuerpo de `soft_limit` (renombrado a `soft_clip`).
- `src/main.rs` o `src/lib.rs` — registrar el módulo nuevo.
- `TODO.md` — marcar items completados, mover a `CHANGELOG.md`.

## Riesgos / tradeoffs

- **Cambio de carácter sónico:** `tanh` empieza a comprimir desde ~0.5 (suave,
  asintótico) vs. el limiter actual que no toca hasta 0.85. Resultado: timbre
  más cálido en niveles altos, posible pérdida de transientes percutivos.
  Mitigación: el usuario ajusta `master_volume`.
- **Doble aplicación de `tanh`:** se preserva por simetría con la implementación
  actual. Si la validación auditiva muestra demasiada compresión, una segunda
  iteración puede aplicar `tanh` solo post-efectos.
- **Fase del HPF:** mínima — fc 5 Hz está muy por debajo del rango audible
  (20 Hz). Inaudible.

## Fuera de scope

- Toggle GUI "vintage saturation" (descartado en brainstorming — sustitución
  directa).
- HPF de mayor orden o variable — primer orden basta para DC.
- Compensación de loudness automática (gain make-up) — el usuario ajusta a oído.
