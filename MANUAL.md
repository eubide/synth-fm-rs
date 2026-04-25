# Manual de Usuario — Emulador FM DX7

## Descripción General

Emulación del Yamaha DX7 / DX7S en Rust: síntesis FM con **6 operadores**,
**32 algoritmos** auténticos, polifonía de **16 voces** y arquitectura
**lock-free** entre los hilos de audio, MIDI y GUI.

Para instalación, requisitos y compilación ver [README.md](README.md).
Para los fundamentos teóricos de la síntesis FM ver [TEORIA-FM.md](TEORIA-FM.md).
Para la arquitectura interna ver [CLAUDE.md](CLAUDE.md).

> **Conceptos universales** (qué es el sonido, MIDI, ADSR, modulación) — están
> bien explicados en [`../synth-analog-rs/TEORIA.md`](../synth-analog-rs/TEORIA.md)
> partes I–II y VI–VIII. Aquí solo cubrimos lo específico del DX7 / FM.

---

## La interfaz: cinco paneles

La pestaña superior selecciona el panel activo:

| Panel | Para qué |
|---|---|
| **VOICE** | Algoritmo, modo de voz, parámetros globales (tune, bend, portamento) |
| **OPERATOR** | Selector de operador (1–6) + todos sus parámetros |
| **LFO** | LFO global, Mod Wheel routing, Pitch EG |
| **EFFECTS** | Chorus / Delay / Reverb (legado reface DX, no DX7) |
| **MIDI** | Canal MIDI, routing de Aftertouch / Breath / Foot, SysEx |

Los cambios se aplican en tiempo real salvo los envelopes: el EG de amplitud
y el Pitch EG se disparan en cada *note-on*, así que para oír un cambio en R1
o L4 hay que pulsar la tecla **después** de mover el slider.

---

## Síntesis FM en 30 segundos

Un **operador** es un oscilador senoidal con su propio envelope. Hay 6.

- Cuando un operador es **carrier** (portador), su salida va al audio.
- Cuando un operador es **modulator** (modulador), su salida modula la frecuencia
  de otro operador. Eso "deforma" la onda senoidal del carrier produciendo
  armónicos.
- El **algoritmo** decide quién es carrier, quién modulator, y cómo se conectan.

Los dos parámetros mágicos:

- **Frequency ratio** del modulator → controla *qué* armónicos aparecen
  (ratios enteros = armónico, fraccionarios = inarmónico/metálico).
- **Output level** del modulator → controla *cuántos* armónicos
  (más nivel = más brillo).

Detalle completo en [TEORIA-FM.md](TEORIA-FM.md).

---

## Modos de Voz (panel VOICE)

| Modo | Descripción |
|------|-------------|
| **Poly** | Hasta 16 voces simultáneas. Robo de la voz más antigua cuando se llena. |
| **Mono** | Una sola voz con portamento continuo entre notas. |
| **M-LEG** *(Mono Legato)* | Mono, pero el envelope NO se redispara mientras haya una nota pulsada — se desliza al pitch nuevo manteniendo el contorno temporal. |

### Parámetros globales del panel VOICE

| Control | Rango | Función |
|---|---|---|
| **Master Volume** | 0.0 – 1.0 | Atenuador final antes del soft-limiter |
| **Master Tune** | ±150 cents | Afinación global de la unidad |
| **Pitch Bend Range** | 0 – 12 semitonos | Rango de la rueda de pitch |
| **Transpose** | ±24 semitonos (C3 = 0) | Desplaza todo el teclado |
| **Portamento Enable** | on/off | Activa el deslizamiento entre notas |
| **Portamento Time** | 0 – 99 | Tiempo de glide (0 ≈ 5 ms, 99 ≈ 2.5 s exponencial) |
| **Glissando** | on/off | El portamento avanza por semitonos discretos en lugar de continuo |

---

## Operadores (panel OPERATOR)

Cada algoritmo dispone los 6 operadores como una pila de carriers (parte
inferior) y modulators (rows superiores). El panel OPERATOR muestra:

1. **Strip selector** arriba: clic en OP1–OP6 para editar uno.
2. **Detalle completo** del operador seleccionado.

### Frecuencia: ratio o fixed

Toggle **RATIO / FIXED**:

- **RATIO**: la frecuencia del operador es `nota_pulsada × frequency_ratio`.
  Rango 0.5 – 31.0. Los ratios enteros (1, 2, 3…) producen sonidos
  armónicos; los fraccionarios (0.5, 1.41, 7.07…) producen sonidos
  inarmónicos como campanas o metálicos.
- **FIXED**: la frecuencia es absoluta en Hz (1 – 4000 Hz), independiente de
  la nota. Útil para "drones" o componentes inarmónicos que no cambian con
  el teclado.

### Output Level (0 – 99)

Su efecto depende del rol del operador en el algoritmo:

- **En carrier** → controla el **volumen** del operador en el mix final.
- **En modulator** → controla el **modulation index** = qué tanto deforma al
  carrier que modula. Subirlo añade armónicos, lo hace más brillante.

Tabla DX7 estándar: cada paso de level = 0.75 dB; level 99 = amplitud máxima
(0 dB), level 0 = silencio.

### Detune (−7 … +7)

Desafinación fina en pasos de aproximadamente 1 cent. Útil para "engrosar"
sumando varios carriers detuneados ligeramente entre sí, o para crear batidos
sutiles entre modulator y carrier.

### Feedback (0 – 7)

**Solo activo en operadores marcados como feedback en el algoritmo** (lo ves
en el diagrama del panel VOICE: el operador con un loop pequeño). Hace que
el operador se module a sí mismo. Niveles bajos (1–3) añaden armónicos
suaves; niveles altos (5–7) lo convierten en ruido casi blanco. La cresta del
sonido `bass eléctrico DX7` es feedback ≈ 6 sobre OP6 en algoritmo 16.

### Velocity Sensitivity (0 – 7)

Cuánto afecta la velocidad de la tecla al output level del operador. 0 = no
responde a velocity; 7 = respuesta máxima. **Truco clave del DX7**: pon
velocity alta SOLO en los modulators y velocity baja en los carriers — así
las notas suaves suenan limpias y las fuertes brillan más (porque entran
más armónicos por el modulator).

### Key Scaling

Tres parámetros que hacen que el operador responda distinto según la zona
del teclado:

- **Breakpoint** (nota MIDI 0 – 127): el "punto de pivote".
- **Left Curve / Right Curve** (LIN+, LIN−, EXP+, EXP−): la forma de
  atenuación a la izquierda y derecha del breakpoint.
- **Left Depth / Right Depth** (0 – 99): cuánto se atenúa en cada lado.

Imita el comportamiento de instrumentos acústicos (un piano suena más
brillante en el grave y más apagado en el agudo, por ejemplo).

### Key Scale Rate (0 – 7)

Cuánto aceleran los rates del envelope a medida que subes en el teclado.
0 = todas las notas tienen la misma velocidad de envelope; 7 = las notas
agudas decaen mucho más rápido (típico de cuerdas y pianos reales).

### AMS — Amp Mod Sensitivity (0 – 3)

Cuánto le afecta la modulación de amplitud del LFO (tremolo) y del EG Bias
del Mod Wheel:

| AMS | Efecto |
|---|---|
| 0 | El operador es inmune al LFO amp y al EG Bias |
| 1 | 9% de profundidad |
| 2 | 37% de profundidad |
| 3 | 100% de profundidad |

(Tabla original DX7 ROM, no lineal.)

### Oscillator Key Sync (on/off)

Si on, la fase del operador se reinicia a 0 en cada *note-on* — sonido
predecible y consistente. Si off, la fase es libre — cada nota es ligeramente
distinta, más "vivo" pero menos repetible. La mayoría de presets DX7 lo
tienen en on.

### Envelope (R1–R4 / L1–L4)

El EG del DX7 **no es ADSR clásico**. Son 4 rates y 4 levels que componen
una trayectoria libre:

```
   L1
    \
     L2          (release a L4 cuando sueltas)
      \         /
       L3 ─sustain
                \
                 L4
```

- **R1** rate de subida hasta L1 desde el reposo (L4).
- **R2** rate desde L1 hasta L2.
- **R3** rate desde L2 hasta L3 → **sustain** mientras la tecla esté pulsada.
- **R4** rate de release hasta L4 al soltar.

Levels 0–99, rates 0–99 (escala logarítmica DX7 ROM). Para emular un ADSR
estándar: L1=99 (ataque a tope), L2=L3=sustain deseado, L4=0.

---

## Algoritmos (panel VOICE — diagrama central)

El **algoritmo** es la topología del patch: define qué operadores son
carriers, cuáles son modulators y cómo se conectan.

Selector 1–32, todos auténticos DX7. Categorías generales:

| Algoritmo | Topología típica | Sonidos típicos |
|---|---|---|
| **1, 2** | 2 carriers + cadenas largas de modulators | Brass complejo, leads densos |
| **3, 4** | Carriers con modulators en paralelo | E-Piano, claves |
| **5** | 3 stacks paralelos de 2 ops | Strings, pads ricos |
| **6, 7, 8** | Cross-feedback | Texturas evolutivas |
| **14, 15** | 2 carriers + cadena | Bell, vibraphone |
| **16, 17** | 1 carrier + pirámide de 5 modulators | Brass agresivo, leads expresivos |
| **18, 19** | Modulator común a varios carriers | E-Piano clásico |
| **20–25** | Mayor cantidad de carriers paralelos | Pads, strings, choir |
| **31** | 5 carriers + 1 modulator | Organ, additive-like |
| **32** | 6 carriers todos en paralelo | Aditiva pura — solo senos sumados |

El panel VOICE muestra el diagrama del algoritmo activo con las conexiones,
columnas, y el feedback loop. Si un operador no está pintado conectado a la
salida, **no se oye** (es modulator).

---

## LFO — Modulación periódica global (panel LFO)

Un único LFO global compartido por todas las voces. Se aplica a la
frecuencia (vibrato) o a la amplitud (tremolo) de los operadores.

### Controles (sub-panel TIMING + MODULATION)

| Control | Rango | Función |
|---|---|---|
| **Rate** | 0 – 99 | Frecuencia del LFO (~0.06 Hz – ~50 Hz) |
| **Delay** | 0 – 99 | Fade-in tras la pulsación de tecla |
| **Pitch Depth** | 0 – 99 | Profundidad del LFO sobre el pitch |
| **Amp Depth** | 0 – 99 | Profundidad del LFO sobre la amplitud |
| **Wave** | TRI / SAW↓ / SAW↑ / SQR / SIN / S&H | Forma de onda |
| **Key Sync** | on/off | Reinicia la fase del LFO en cada nota |

### Mod Wheel Routing

Los tres sliders bajo "MOD WHEEL ROUTING" deciden cómo el Mod Wheel
(CC1) afecta a la modulación. Cada uno es 0 – 7:

| Sensitivity | Destino |
|---|---|
| **PMS** (Pitch Mod Sens) | Escala del LFO Pitch — el wheel "mete vibrato" |
| **EG Bias** | Atenuación estática a operadores con AMS > 0 |
| **P-Bias** (Pitch Bias) | Offset estático de pitch (±2 semitonos al máximo) |

`PMS` usa la tabla DX7 ROM no lineal `[0, 0.082, 0.16, 0.32, 0.5, 0.79, 1.26, 2.0]`.
A PMS=7 con LFO Pitch Depth=99, el wheel al máximo da ±2 semitonos
de oscilación — vibrato amplio. PMS=3 = vibrato suave (~½ semitono).

### Pitch EG

El **Pitch Envelope Generator** es independiente del EG de amplitud y aplica
un offset de pitch global a la voz durante la nota. Útil para *attacks* tipo
brass (la nota "sube" al pitch correcto en los primeros 50 ms) o *drops* en
release.

| Control | Rango | Función |
|---|---|---|
| **enabled** | on/off | Bypass total cuando off |
| **R1–R4** | 0 – 99 | Rates entre stages, igual que el EG de amplitud |
| **L1–L4** | 0 – 99 | Levels donde **50 = sin offset**, 0 ≈ −4 oct, 99 ≈ +4 oct |

Trayectoria: arranca en L4 → sube/baja a L1 a R1 → a L2 a R2 → sustain en L3
con R3 → release a L4 a R4. **Recordatorio importante: L4 es a la vez la
posición de reposo Y el destino del release.**

### Mod Wheel actual

Bajo el separador final, el panel muestra el valor actual del Mod Wheel
(0 – 100%). Si está en 0, los routings de PMS/EG-Bias/P-Bias no producen
efecto audible aunque las sensibilidades estén altas.

---

## Controladores externos (panel MIDI)

El DX7 / DX7S enruta varios controladores físicos a la misma matriz de
4 destinos (PITCH / AMP / EG_BIAS / PITCH_BIAS), cada uno con sensitivity
0 – 7 independiente. Esto es lo que hace al DX7 expresivo más allá del
teclado.

### MIDI INPUT CHANNEL

Combo selector: **OMNI** (todos los canales) o **Ch 1 – 16**. Mensajes de
sistema (SysEx) bypasean el filtro siempre. Cambia en caliente sin cortar
las notas activas.

### AFTERTOUCH (0xD0)

Channel pressure — la presión que ejerces sobre la tecla **después** de
pulsarla. Cuatro destinos 0 – 7:

| Destino | Efecto al apretar |
|---|---|
| **PITCH** | Refuerza la profundidad del LFO de pitch (vibrato bajo presión) |
| **AMP** | Refuerza el LFO de amplitud (tremolo bajo presión) |
| **EG-BIAS** | Atenúa los operadores con AMS > 0 (timbre se "apaga") |
| **P-BIAS** | Bend estático de pitch hasta ±2 semitonos |

El indicador "input: NN%" muestra la presión actual.

### BREATH CTRL (CC2)

Controlador de aliento — un sensor que mide tu respiración. Misma matriz
de 4 destinos. Históricamente muy usado en patches de viento (saxos, flautas)
para que la dinámica siga al fuelle del intérprete.

### FOOT CTRL (CC4)

Pedal de expresión. Cuatro destinos: **VOLUME (0 – 15)**, **PITCH**, **AMP**,
**EG-BIAS** (no tiene PITCH_BIAS).

`VOLUME` es especial: actúa como **swell pedal** — sens=0 ignora el pedal,
sens=15 silencia completamente cuando el pedal está al mínimo. Útil en
secciones tipo cuerda con dinámica de pedal.

### Otros mensajes MIDI soportados

| Mensaje | CC | Función |
|---|---|---|
| Note On / Off | — | Reproducción con velocity 0 – 127 |
| Pitch Bend | — | Rango configurable en el panel VOICE |
| Mod Wheel | CC1 | Profundidad LFO + EG/Pitch Bias |
| Sustain Pedal | CC64 | ≥64 mantiene notas sin liberar EG |
| Expression | CC11 | Atenuador genérico multiplicativo |
| Bank Select MSB | CC0 | Combinado con CC32 + Program Change |
| Bank Select LSB | CC32 | Bits bajos del banco |
| Program Change | — | Carga preset = `(MSB<<14)|(LSB<<7)|program` |
| All Notes Off | CC123 | Panic |
| SysEx | — | Carga voz simple (155 B) o bulk 32 voces (4096 B) |

---

## SysEx (sub-panel del MIDI)

Compatible con el formato VCED / VMEM original del DX7 — puedes intercambiar
patches con hardware DX7 real o con cualquier editor SysEx (Dexed, librarian
clásicos, archivos `.syx` de internet).

| Acción | Cómo |
|---|---|
| **Load .syx** | Escribe la ruta del archivo, clic en *Load .syx*. Detecta si es single voice o bulk dump y aplica/sustituye el banco automáticamente. |
| **Save current voice** | Escribe la ruta destino y clic en *Save current voice*. Exporta la voz activa como VCED de 163 bytes (con checksum). |

El estado del último intento aparece en gris debajo de los botones.

---

## Efectos (panel EFFECTS — herencia reface DX)

> **Nota de autenticidad**: ni el DX7 ni el DX7S original incluyen efectos
> internos. La cadena `Chorus → Delay → Reverb` es herencia del reface DX
> (2015) y existía antes de aplicar la política de autenticidad actual.
> Se mantiene como utilidad pero no es DX7-puro. Ver [TODO.md sección 3](TODO.md).

### Chorus

| Control | Rango | Función |
|---|---|---|
| **Enabled** | on/off | |
| **Rate** | 0.1 – 5.0 Hz | Velocidad de la modulación |
| **Depth** | 0.0 – 10.0 ms | Profundidad del retardo modulado |
| **Mix** | 0 – 1 | Wet/dry |
| **Feedback** | 0 – 0.7 | Recursión interna |

### Delay

| Control | Rango | Función |
|---|---|---|
| **Time** | 0 – 1000 ms | Tiempo del retardo |
| **Feedback** | 0 – 0.9 | Repeticiones |
| **Mix** | 0 – 1 | Wet/dry |

### Reverb (Schroeder)

| Control | Rango | Función |
|---|---|---|
| **Room Size** | 0 – 1 | Tamaño percibido del espacio |
| **Damping** | 0 – 1 | Atenuación de agudos en cada reflexión |
| **Mix** | 0 – 1 | Wet/dry |
| **Width** | 0 – 1 | Apertura estéreo |

---

## Sistema de Presets

### ROM hardcoded (32 voces)

Los 32 presets clásicos de la **ROM 1A** del DX7 original están cargados en
memoria al arrancar. Selección vía Program Change MIDI (programa 0 = preset 0).

### Colecciones JSON externas

`patches/` contiene subdirectorios; cada subdirectorio es una **colección**:

| Colección | Origen | Notas |
|---|---|---|
| `mark/` | itsjoesullivan/dx7-patches | 26 patches de alta calidad |
| `edu/` | propios | Patches construidos como ejemplos didácticos |

El loader ([`preset_loader.rs`](src/preset_loader.rs)) escanea recursivamente
y los expone en la GUI con filtro por colección. Formato JSON compatible con
itsjoesullivan/dx7-patches; los detalles del parseo (renombres `pitchEG`,
`amDepth` int|string, breakpoints, tablas AMS/PMS ROM) viven en el propio
módulo `preset_loader.rs`.

---

## Controles del Teclado de Computadora

```
Notas (octava media):

A  W  S  E  D  F  T  G  Y  H  U  J  K  O  L  P  Ñ
C  C# D  D# E  F  F# G  G# A  A# B  C  C# D  D# E
```

Octava: flechas arriba / abajo.

---

## Ejemplos: cómo construir patches paso a paso

### 1. Brass clásico — algoritmo 16

Receta: 1 carrier + 5 modulators en pirámide, ataque rápido, ratios enteros.

1. Algoritmo: **16**
2. OP1 (carrier): ratio = 1.0, level = 99, EG: R1=99 R2=80 R3=50 R4=80, L1=99 L2=85 L3=70 L4=0
3. OP2 (modulator de OP1): ratio = 1.0, level = 80, mismo EG aproximado
4. OP3 – OP6: ratios enteros (2, 3, 1, 1), levels descendentes 70, 60, 50, 40 — añaden armónicos
5. **Pitch EG**: enabled, L4=47 (≈3 semitonos abajo), R1=80, resto 50 / 99 — el "blow" característico
6. AMS=2 en OP1, LFO Triangle Rate=40 — vibrato sutil con Mod Wheel

Cómo se oye: ataque claro, brillo medio, sustain sin decay, release corto.

### 2. E-Piano DX7 — algoritmo 5

Receta: 3 stacks paralelos de 2 ops, modulators con decay rápido para que
el timbre cambie tras el ataque.

1. Algoritmo: **5**
2. OP1 / OP3 / OP5 (carriers): ratio = 1.0, level = 99 / 90 / 80
3. OP2 / OP4 / OP6 (modulators): ratio = 14.0, 1.0, 1.0; level = 75
4. EG modulator: R1=99 R2=70 R3=20 R4=70, L1=99 L2=50 L3=0 L4=0 — decay rápido
5. EG carrier: R1=99 R2=50 R3=10 R4=70, L1=99 L2=80 L3=0 L4=0 — sostén corto, decay tipo cuerda piano
6. Velocity sensitivity = 7 en OP2 — el ataque es más metálico cuanto más fuerte la nota

Cómo se oye: "ding" inicial brillante seguido de un cuerpo más limpio que
decae como un piano eléctrico.

### 3. Bell metálico — algoritmo 1

Receta: ratios fraccionarios = inarmónico = sonido de campana.

1. Algoritmo: **1**
2. OP1 (carrier): ratio = 1.0, level = 99
3. OP2 (modulator de OP1): ratio = **1.41** (≈ √2 — fundamentalmente inarmónico), level = 80
4. OP3 (modulator de OP2): ratio = **3.5**, level = 60
5. EG: ataque instantáneo (R1=99, L1=99), decay largo (R2=30 L2=30, R3=20 L3=0)
6. Sin sustain — la nota debe extinguirse sola

Cómo se oye: campana brillante con armónicos no enteros que producen el
"shimmer" característico del DX7 (TUBULAR BELLS, GLASS, BELL).

### 4. Pad evolutivo

Receta: LFO suave modulando un modulator → el timbre cambia con el tiempo.

1. Algoritmo: **22** (4 carriers paralelos + 2 modulators)
2. Carriers: ratios 1, 1, 2, 0.5 (octava abajo) — armónicos amplios
3. Modulators: ratio 7, 3 — sutiles pero presentes (level 30)
4. EG carriers: R1=20 R2=20 R3=99 R4=20, L1=99 L2=99 L3=99 L4=0 — ataque y release lentos
5. LFO: Triangle, Rate=12, Pitch Depth=15 — vibrato muy lento
6. Pitch Bend Range = 12 — para dramatizar
7. Reverb: room 0.7, mix 0.5 (sí, es legacy reface, pero queda bien)

Cómo se oye: pad que respira lentamente, con un crecimiento de timbre durante
los primeros segundos.

### 5. Bass percusivo — algoritmo 32

Receta: aditiva pura. Solo senos sumados, EG percusivo.

1. Algoritmo: **32** (los 6 ops son carriers)
2. Ratios: 1, 2, 3, 4, 5, 6 (serie armónica)
3. Levels: 99, 70, 50, 30, 20, 10 — decreciente (sonido tipo "saw aproximada")
4. EG idéntico en los 6: R1=99 R2=70 R3=20 R4=80, L1=99 L2=70 L3=0 L4=0
5. Velocity sensitivity = 4 en todos — punche en notas fuertes

Cómo se oye: bass nítido, con cuerpo armónico, decay rápido — ideal para
funk o slap programado.

---

## Técnicas avanzadas

### Cómo hacer un patch desde cero (workflow recomendado)

1. **Selecciona el algoritmo** según la *topología* deseada (un carrier
   simple + cadena = lead; varios carriers = pad; aditiva = bass armónico).
2. **Ratios primero**: empieza con todos los modulators a ratio = 1.0.
   Mueve uno de cada vez para entender cómo cambia el timbre. Ratios enteros
   = sonidos "de instrumento"; fraccionarios (0.5, 1.41, π…) = inarmónicos.
3. **Output levels después**: sube los modulators hasta que oigas el brillo
   deseado (típicamente 60 – 85). Bájalos si suena agresivo.
4. **Envelopes de los modulators**: define la *evolución del timbre*. Decay
   rápido en modulator = "ding" seguido de cuerpo limpio. Sustain alto =
   timbre estable. Decay lento = evolución del color.
5. **Envelope del carrier**: el contorno de volumen tradicional (ataque,
   sustain, release).
6. **Velocity en modulators**: para que las notas fuertes brillen más sin
   afectar el volumen.
7. **AMS / LFO** al final: vibrato y tremolo.

### Feedback como fuente de aspereza

El operador con feedback en el algoritmo activo ejerce un papel especial:
con feedback = 0 produce un seno limpio; con feedback ≈ 4 empieza a sonar a
sierra; con feedback ≥ 6 a ruido casi blanco. Útil para:

- Bajos eléctricos: feedback ≈ 5 sobre OP6 carrier en algoritmo 31.
- Hi-hats / cymbals: feedback = 7 sobre operador inarmónico.
- Strings vintage: feedback ≈ 2 sobre el modulator superior.

### Modulación inarmónica controlada

Si quieres un sonido **casi armónico pero con tensión** (típico de leads
expresivos), prueba ratios cercanos a enteros pero ligeramente desplazados:
1.01, 2.02, 3.04. El batido entre el carrier y el modulator produce un
"shimmer" sutil que en ratios enteros perfectos no aparece.

### Mod Wheel expresivo en stacks paralelos

En algoritmos con múltiples carriers paralelos (5, 22, 31, 32), pon AMS = 3
solo en uno o dos de ellos. Al mover el Mod Wheel con `EG Bias` alto,
solo esos operadores se atenúan, **cambiando el timbre relativo** del patch
en directo. Es la diferencia entre un instrumento "vivo" y uno estático.

### Pitch EG para bends de instrumentos acústicos

- **Brass attack swoop**: L4=47, R1=80, resto=50/99. Pitch sube ~3 semitonos
  en 50 ms al pulsar.
- **Slide-down en release**: L4=30, R1=99 (ataque instantáneo en pitch),
  R4=40 (release lento que cae 16 semitonos). Pitch cae al soltar.
- **Pad sweep**: L4=38, R1=30 (~1s para subir), R4=70. La nota arranca
  abajo y trepa al pitch durante el ataque.

---

## Solución de problemas

### Audio

- **Sin sonido tras cargar un preset**: comprueba que en el algoritmo activo
  hay al menos un *carrier* con output level > 0. Si solo hay modulators
  audibles (porque cambiaste el algoritmo a uno con menos carriers), no
  pasa nada al amplificador.
- **Sonido apagado**: sube el output level de los modulators del algoritmo
  o reduce el feedback (si está alto, está enmascarando los armónicos
  útiles con ruido).
- **CLIP / distorsión**: baja Master Volume; el soft-limiter actúa pero
  todavía colorea cuando el bus se satura.

### Cambios al envelope que no se oyen

El EG de amplitud y el Pitch EG se disparan en cada *note-on*. Si modificas
R1 / L1 mientras una nota está sonando, el cambio se oye en la **siguiente
pulsación**, no en la nota activa. Para depurar envelope, pulsa-suelta-pulsa
después de cada cambio.

### MIDI no detectado

Conecta el dispositivo MIDI **antes** de arrancar el binario. El handler
abre la primera puerta de entrada disponible al iniciar y no escanea de
nuevo durante la ejecución.

### Aftertouch / Breath / Foot no responden

1. Verifica que el dispositivo realmente envía esos mensajes (algunos
   teclados envían aftertouch poly en vez de channel — el DX7 solo entiende
   channel pressure, status `0xD0`).
2. Asegúrate de que las sensibilidades correspondientes en el panel MIDI
   están > 0. Por defecto todas están a 0.
3. Mira el indicador "input: NN%" en el panel — si no se mueve al usar el
   controlador, no está llegando el mensaje.

### Patches importados desde Dexed / .syx suenan distinto

Causas comunes:

- Algunos parámetros (especialmente en SysEx VCED) tienen rangos restringidos
  a 0 – 99; otras implementaciones reescalan. La nuestra respeta los
  valores DX7 originales.
- Si el patch usa **PMS** o **AMS** altos, el efecto se nota mucho con el
  Mod Wheel arriba — comprueba la posición del wheel.
- Reface DX–style features (efectos por preset, 7 tipos de FX) no están
  implementados; el archivo carga pero los efectos quedan en el global
  preexistente.

---

## Convenciones DX7 que vale la pena recordar

- **Levels y rates 0 – 99**: nunca 0 – 100 ni 0 – 127. Es la cuadrícula
  original DX7.
- **Algoritmo 1-indexed**: el panel y la documentación usan algoritmo 1 – 32,
  no 0 – 31, igual que el manual original.
- **OP1 abajo, OP6 arriba** en el diagrama: convención DX7. OP6 suele ser
  el modulator más agudo y/o el operador con feedback.
- **Level 50 ≠ silencio en Pitch EG**, sí en EG de amplitud. En Pitch EG,
  50 = "no offset", convención DX7 ROM.

---

## Lectura adicional

- **Manual original DX7S** (PDF en `_docs.md/DX7S.pdf`): la fuente canónica.
- **Synth Secrets — Sound on Sound** (FM series): magníficos artículos
  pedagógicos online, especialmente "Synth Secrets parts 12, 13, 14".
- **Chowning, J. M. (1973)** *The Synthesis of Complex Audio Spectra by
  Means of Frequency Modulation*: el paper original.
- **TEORIA del proyecto hermano** ([../synth-analog-rs/TEORIA.md](../synth-analog-rs/TEORIA.md)):
  partes I–II y VIII para fundamentos universales (sonido, MIDI, modulación).
