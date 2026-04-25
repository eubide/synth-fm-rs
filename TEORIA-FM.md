# Síntesis FM — Fundamentos Específicos del DX7

## Por qué este documento existe

El proyecto hermano [`synth-analog-rs`](../synth-analog-rs/TEORIA.md) cubre
los fundamentos universales: qué es el sonido, MIDI, decibeles, Fourier,
envelopes, LFO, modulación. **No los repetimos aquí.**

Lo que sí repetimos NO está allí: por qué la síntesis FM existe, qué es un
operador, qué es la modulation index, por qué el DX7 suena a campana o a
piano eléctrico, y cómo se piensa un patch FM desde cero.

> Si nunca has trabajado con MIDI ni con sintetizadores, lee primero las
> partes I (El Sonido), II (MIDI) y VIII (LFO/Modulación) del TEORIA del
> hermano. Después vuelve aquí.

Este documento es **complementario al [MANUAL.md](MANUAL.md)**: el manual
explica *qué hace cada slider*, esto explica *por qué hace lo que hace*.

---

## PARTE I — Por qué FM, y por qué en 1983

### 1. El problema que resolvía la FM

En 1980, hacer un sonido de **campana** o de **piano eléctrico** convincente
con síntesis sustractiva era casi imposible. Los timbres metálicos llevan
docenas de armónicos no enteros (ratios `1.41`, `2.78`, etc. respecto del
fundamental) que no aparecen en una sierra ni en un cuadrado, ni los
sintetiza un filtro paso-bajo.

La síntesis aditiva podía hacerlo en teoría, sumando 30 osciladores
sinusoidales — pero un sintetizador con 30 osciladores por voz era
inviable comercialmente.

**John Chowning** descubrió en Stanford (1967, publicado 1973) que
modulando la frecuencia de un seno con otro seno, se generan automáticamente
muchos armónicos a la vez, controlables con solo dos parámetros.

Yamaha licenció la patente y en 1983 sacó el DX7 con 6 operadores. Costaba
¢ 2,000 USD, vendió más de 200,000 unidades, y definió el sonido de los 80s.

### 2. La intuición central, en una analogía

| Sustractiva | FM |
|---|---|
| Empiezas con una onda compleja (sierra, cuadrado) y **quitas** armónicos con un filtro. | Empiezas con un seno puro y **generas** armónicos modulando su frecuencia. |
| "Tallar una estatua de mármol con un cincel." | "Hacer una rueda girar dentro de otra rueda — el patrón resultante depende del ratio entre ambas." |
| Parámetros principales: cutoff y resonance. | Parámetros principales: ratio y modulation index. |

Si estás acostumbrado al filtro Moog: en sustractiva subir el cutoff = más
brillo. En FM subir el output level del *modulator* = más brillo. La
analogía funcional aguanta sorprendentemente bien.

### 3. Por qué fue revolucionario

- Sonidos imposibles en sustractiva: bell, mallet, vocal, electric piano.
- 6 osciladores producen el equivalente tímbrico de 30 osciladores aditivos.
- Predecible: el mismo ratio + mismo level siempre da el mismo timbre
  (a diferencia del filtro analógico, que envejece).
- 32 algoritmos pre-cableados ahorran al usuario diseñar topologías a mano.

---

## PARTE II — Anatomía del operador

### 4. Qué es un operador

Un **operador** = un seno + su envelope:

```
   ┌────────────────────────────────┐
   │  Operador                       │
   │                                 │
   │   [Frecuencia] ──→ sin(2πft)    │
   │                       │         │
   │   [EG: R1..R4,        │         │
   │       L1..L4]   ──→  ×          │
   │                       │         │
   │   [Output level]──→  ×          │
   │                       │         │
   │   [Modulation in]→ +  │         │
   │                       ↓         │
   │                   salida        │
   └────────────────────────────────┘
```

Comparado con la cadena VCO + VCA del Prophet:

| Componente analógico | Equivalente DX7 |
|---|---|
| VCO (oscilador) | Operador (siempre seno) |
| VCA (amplificador) | Envelope ya incluido en el operador |
| Filtro | **No existe.** El "filtrado" es el modulation index. |
| Envelope dedicado | Cada operador tiene el suyo, no compartido |

Cada operador es **autosuficiente**: no necesita pasar por nada externo.
Por eso 6 operadores producen un patch completo sin un VCF al final.

### 5. Carrier vs Modulator — el rol depende del algoritmo

Un mismo operador puede ser carrier en un algoritmo y modulator en otro.
**No es una propiedad del operador, es del cableado.**

- **Carrier** ("portador"): su salida va al audio final. Si pongo level=0,
  desaparece del patch.
- **Modulator**: su salida modula la frecuencia de otro operador. Si pongo
  level=0, el carrier que modula vuelve a ser un seno puro.

En el panel VOICE del emulador, el **diagrama del algoritmo** muestra esto
claro: los operadores conectados a la línea horizontal de abajo son
carriers; los apilados encima son modulators que apuntan a quien tienen
debajo.

### 6. La cadena básica: modulator → carrier

El bloque elemental de la FM es **un par**:

```
   [Modulator]
        │
        ↓ (modula la frecuencia de…)
   [Carrier]
        │
        ↓
     audio
```

Con esto solo, controlando dos parámetros (ratio del modulator, output level
del modulator), generas:

- Si **ratio = 1** y level = 0 → seno puro.
- Si **ratio = 1** y level alto → onda parecida a sierra (armónicos enteros).
- Si **ratio = 0.5** → octava abajo, timbre con sub-armónicos.
- Si **ratio = 1.41** → campana (inarmónico).
- Si **ratio = 14** → piano eléctrico (armónicos muy altos sutiles).

Toda la FM se construye apilando y combinando estas parejas.

---

## PARTE III — Los dos parámetros mágicos

### 7. Modulation index — "cuánta FM"

Cuando un modulator modifica la frecuencia de un carrier, lo hace en un
rango que depende de la **amplitud del modulator** (su output level):

```
Modulation index ≈ amplitud_modulator / frecuencia_modulator
```

En el DX7 esto se controla directamente con el **output level del modulator
(0 – 99)**. La escala interna usa `4π` como factor (ver
[CLAUDE.md](CLAUDE.md) → "Critical FM Synthesis Details") para producir el
rango auténtico DX7: hasta ~12.57 radianes a level=99.

**Lo que se oye**:

- Modulation index = 0 → carrier es un seno puro (ningún armónico extra).
- Modulation index ≈ 1 → 2-3 armónicos audibles (sonido "musical, redondo").
- Modulation index ≈ 3 → 5-7 armónicos (sonido brillante, "boca abierta").
- Modulation index ≈ 10 → ~15 armónicos (sonido estridente o muy metálico).
- Modulation index ≥ 15 → ruido cuasi-blanco (la FM colapsa en aliasing).

Regla mental: **subir el output level del modulator = subir el cutoff del
filtro en sustractiva**. Más nivel = más brillo. Es la equivalencia más
útil que puedes interiorizar.

### 8. Frequency ratio — "qué armónicos"

Si el modulation index decide *cuántos* armónicos aparecen, el **frequency
ratio** decide *cuáles*. La fórmula matemática (sin demostrar) dice que
modular un seno de frecuencia `fc` con otro de frecuencia `fm` produce
energía a:

```
fc, fc±fm, fc±2fm, fc±3fm, …
```

(Eso son los famosos *sidebands*.) La intuición práctica:

| Ratio modulator/carrier | Sidebands resultantes | Carácter |
|---|---|---|
| **1.0** | fc, 2fc, 3fc, 4fc, … (serie armónica completa) | Tipo sierra, instrumentos de viento |
| **2.0** | fc, 3fc, 5fc, 7fc, … (impares) | Tipo cuadrada, clarinete, oboe |
| **3.0** | fc, 2fc, 4fc, 5fc, … (con saltos) | Lead expresivo, brass |
| **0.5** | fc, fc/2, 3fc/2, 2fc, … (sub-armónicos) | Bajo gordo, octava extra |
| **14** | fc, ~14fc, ~15fc, ~13fc | "Click" inicial brillante (típico E-Piano) |
| **1.41** *(≈√2)* | fc, fc±1.41fc, fc±2.82fc, … (no enteros) | Campana, glass, bell |
| **3.5** | mezcla inarmónica densa | Mallet, marimba, vibraphone |

**Regla de oro**: ratios enteros (`1, 2, 3, 4, 5, 6`) = sonidos "de
instrumento real" (notas con armónicos coherentes). Ratios fraccionarios
no enteros (`1.41, 3.5, 7.07`) = sonidos metálicos / inarmónicos. Esto
es **la firma sonora del DX7** — el repertorio de campanas, vidrios y
mallets que dominó toda la música pop de los 80.

### 9. Bessel sin matemáticas

La distribución de energía entre los sidebands sigue las **funciones de
Bessel**, una familia de curvas oscilatorias que dependen del modulation
index. No las vas a calcular a mano, pero sí conviene retener el
comportamiento cualitativo:

- Modulation index bajo → casi toda la energía está en el carrier.
  Suena casi a seno.
- Modulation index medio → la energía se reparte entre carrier y los
  primeros 3-5 sidebands. Suena rico pero ordenado.
- Modulation index alto → la energía sale del carrier hacia los
  sidebands altos. **El fundamental se debilita**, lo que da el sonido
  "vacío en el medio" típico de patches DX7 con muchos modulators activos.
- Modulation index muy alto → energía dispersa por todo el espectro. Ruido.

Esto explica por qué un patch FM mal configurado puede sonar "delgado": si
los modulators están demasiado altos, el carrier original (la nota que
querías oír) se reparte tanto que ya casi no se percibe el pitch.

---

## PARTE IV — Los 32 algoritmos

### 10. Por qué hay tantos

Cada algoritmo es una **topología de cableado** distinta entre los 6
operadores. El DX7 tiene 32 algoritmos seleccionables (todos cargados de ROM
en este emulador, ver `algorithms.rs`).

¿Por qué 32 y no 1? Porque la organización jerárquica importa:

- **Algoritmo 1** (1 carrier + cadena de 5 modulators): un solo carrier muy
  modulado → sonido único y denso.
- **Algoritmo 32** (6 carriers en paralelo, ningún modulator): aditiva
  pura → 6 senos sumados.
- **Algoritmo 5** (3 stacks paralelos de carrier + modulator): 3 timbres
  distintos en paralelo → strings, choirs, pads.
- **Algoritmo 16** (1 carrier + 5 modulators ramificados): brass agresivo
  con mucho ataque.

Cambiar de algoritmo es cambiar **toda la arquitectura** del sonido — más
parecido a cambiar de instrumento que a girar un parámetro.

### 11. Cómo elegir un algoritmo

| Quieres construir… | Familia de algoritmos buena |
|---|---|
| Bass redondo + click | 31, 32 (varios carriers paralelos) |
| Strings o pads | 5, 22, 27, 28 (carriers paralelos con modulator común) |
| Brass agresivo, lead expresivo | 16, 18, 19 (1 carrier + cadena modulator) |
| E-Piano | 5, 18 (decay rápido en modulators) |
| Bell, vibraphone, mallet | 1, 14, 15 (cadenas largas, ratios fraccionarios) |
| Organ, additive-like | 31, 32 (todos carriers) |
| Texturas evolutivas / FX | 4, 6 (cross-feedback en operadores) |

Truco práctico: empieza con el algoritmo cuyo diagrama "más se parece" a
la categoría del sonido. La elección final se hace tocando.

### 12. Lectura del diagrama

```
       OP6
        │
       OP5
        │           OP3
       OP4           │
        │           OP2
       OP1           │
        ╧═══════════╧═══  ← línea de carriers
              ↓
            audio
```

Convención DX7 que respeta el emulador:

- **OP1 abajo**, OP6 arriba. La línea horizontal inferior es la salida.
- Los operadores **conectados directamente** a la línea son **carriers**.
- Los operadores **apilados encima** son modulators que apuntan a su
  vecino inmediato inferior.
- Si dos operadores comparten "padre" (uno mismo es modulado por dos), se
  ramifica.
- El **bucle pequeño** sobre un operador indica feedback (típicamente
  OP6).

El diagrama del panel VOICE muestra esto en tiempo real al cambiar el
algoritmo.

---

## PARTE V — Feedback

### 13. Feedback en FM ≠ feedback de filtro

En sustractiva, "feedback" en un filtro Moog = realimentación que produce
**resonancia** (un pico afilado en el cutoff). En FM, "feedback" es algo
muy distinto: el operador se **modula a sí mismo**.

```
   ┌─────────┐
   │ Op feedback│←──┐
   └─────────┘   │
        │        │
        └────────┘ (la salida se reinyecta como modulación)
```

El efecto: la onda senoidal se transforma progresivamente en otras formas
de onda según la cantidad de feedback.

### 14. Feedback como "control de aspereza"

| Feedback (0–7) | Forma de onda resultante |
|---|---|
| 0 | Seno puro |
| 1–2 | Algo parecido a triángulo |
| 3–4 | Hacia sierra (armónicos enteros crecientes) |
| 5–6 | Sierra muy brillante con tinte ruidoso |
| 7 | Ruido casi blanco |

Aplicaciones típicas:

- **Bass eléctrico**: feedback ≈ 5 sobre OP6 carrier → cuerpo grueso con
  graspy.
- **Hi-hat / cymbal**: feedback = 7 sobre operador inarmónico → ruido
  controlable.
- **Strings vintage**: feedback ≈ 2 sobre el modulator más alto → calor
  sutil sin perder armonicidad.

### 15. Cross-feedback — algoritmos 4 y 6

Dos algoritmos del DX7 tienen **feedback cruzado** entre dos operadores
(no auto-feedback): la salida de uno modula al otro Y al revés. Produce
texturas evolutivas, casi caóticas, que se prestan a FX y SoundFX.

En el código (`operator.rs`), esto se maneja con el método
`cross_feedback_signal()` y un escalado distinto al del self-feedback,
para evitar que la realimentación cruzada se dispare a niveles ruidosos.

---

## PARTE VI — Por qué DX7 suena DX7

### 16. Anatomía de un sonido tipo "BELL"

Tomemos el preset clásico TUBULAR BELLS:

- Algoritmo 1 → 1 carrier + cadena
- OP1 carrier, ratio = 1.0, level = 99
- OP2 modulator, ratio = **1.41**, level = 80, decay rápido
- OP3 modulator de OP2, ratio = **3.5**, level = 60, decay larguísimo
- OP4–OP6: ratios enteros bajos, levels bajos, contribuyen sutilezas
- Sin sustain — la nota se extingue sola

Lo que oyes:

- **Ataque**: el modulator OP2 al máximo level produce un pico de
  modulation index → muchos sidebands inarmónicos altos.
- **Decay rápido del OP2**: los sidebands se desvanecen → el sonido
  "se limpia" hacia un seno casi puro.
- **OP3 con decay largo**: añade un *aura* metálica que sostiene la cola.
- **Ratios 1.41 y 3.5**: el espectro NO es múltiplo entero del fundamental,
  por lo que la oreja no percibe "una nota con armónicos" sino "un
  conjunto de tonos relacionados", como una campana real.

Esto es **imposible** de hacer con un filtro paso-bajo. La FM da acceso
directo a esta clase tímbrica.

### 17. Anatomía de un sonido tipo "E-PIANO"

Preset clásico tipo DX7 RHODES:

- Algoritmo 5 → 3 stacks paralelos de carrier + modulator
- Carriers: ratios 1, 1, 1 (todos al unísono)
- Modulators: ratios 14, 1, 1 (uno con click muy alto, dos suaves)
- Velocity sensitivity = 7 en los modulators, 0 en los carriers

Lo que oyes:

- **Ataque**: el modulator con ratio 14 produce un sideband alto que se
  oye como "click" o "tine" (la lengüeta metálica del Rhodes real).
- **Velocity sensitivity en el modulator**: las notas fuertes generan más
  click; las suaves quedan limpias y dulces.
- **Decay rápido del modulator**: el click desaparece y queda solo el
  cuerpo armónico.
- **Cuerpo armónico**: 3 carriers paralelos a ratios cercanos = chorus
  natural.

Resultado: el sonido cambia con la velocity como un piano real. El "Rhodes
DX7" definió el sonido de baladas, jazz fusion, smooth jazz de los 80.

### 18. Por qué los presets DX7 envejecieron como envejecieron

El DX7 original tenía:

- Convertidor D/A de 12 bits con companding µ-law → calidad media-baja con
  un tinte característico (ese "grain" que algunos buscan).
- Sample rate efectivo de unos 49 kHz → algo de aliasing en agudos.
- Polifonía limitada (16 voces).

Nuestra emulación es **más limpia** (CPAL, 44.1 / 48 kHz, f32). Los patches
suenan más prístinos pero pierden algo del "carácter sucio" del hardware.
Para recuperarlo se podría añadir un "DX7 mode" con bit-crush + companding,
pero por ahora no está implementado (ver
[TODO.md sección 8](TODO.md) → soft clipper).

---

## PARTE VII — Cuadro mental de referencia

### 19. Tabla "qué quiero oír → cómo lo monto"

| Quiero… | Algoritmo sugerido | Ratios típicos | Output level modulator | Envelope clave |
|---|---|---|---|---|
| Brass clásico | 16, 18 | enteros (1, 2, 3) | 70–85 | EG carrier sostenido, modulator con decay medio |
| E-Piano | 5, 18 | 14 + 1, 1 | 60–80 | Modulator decay muy rápido (R2=70, L2=0) |
| Bell, vibraphone | 1, 14 | fraccionarios (1.41, 3.5) | 70–90 | Carrier sin sustain, decay largo |
| Lead expresivo | 16 | 1, 1, 2 | 80–95 | Carrier sostenido, AMS=2, vibrato MW |
| Pad evolutivo | 22, 27 | 1, 2, 1, 0.5 | 30–60 | Ataques lentos, LFO modulando level |
| Bass percusivo | 31, 32 | enteros bajos (1, 2, 3) | 70–99 | Decay rápido, velocity en carriers |
| Strings | 5, 22 | 1 + 1 (un poco detune) | 50–70 | Ataque medio, sustain alto, release medio |
| Choir / vocal | 22, 28 | enteros con mucho stack | 50–80 | LFO Pitch sutil, velocity baja |
| FX evolutivo | 4, 6 (cross-feedback) | mezcla inarmónica | 60–95 | LFO modulando algo |
| Drum / mallet | 14, 15 | fraccionarios | 80–99 | Sin sustain, R3=0 |

### 20. Decisiones de diseño DX7 que vale la pena conocer

Estas son convenciones DX7 que el emulador respeta y que conviene tener en
mente al programar patches:

- **Levels y rates son 0 – 99**, no 0 – 127. Es la cuadrícula original Yamaha.
- **Output level 99 = 0 dB**, cada paso es 0.75 dB de atenuación.
- **L4 es el "reposo"** del envelope: es donde arranca la nota y donde
  termina el release.
- **Algoritmos 1-indexed** (1 – 32, no 0 – 31), aunque en el código interno
  son 0 – 31.
- **OP6 suele ser feedback** por convención de los presets ROM, pero
  cualquier operador puede tener feedback en algoritmos donde aplique.

### 21. Conceptos para retener

Si solo te llevas tres ideas:

1. **Output level del modulator = "cutoff" de la sustractiva.** Más nivel =
   más brillo, menos nivel = más limpio.
2. **Frequency ratio decide el carácter** (entero = armónico, fraccionario =
   metálico/inarmónico). No el filtro. No la forma de onda. El ratio.
3. **El envelope del modulator define cómo evoluciona el timbre en el
   tiempo**, no solo el volumen. Decay rápido en modulator = ataque
   brillante seguido de cuerpo limpio. Sustain alto = timbre estable.

---

## Lectura adicional

- **Chowning, J. M. (1973)** — *The Synthesis of Complex Audio Spectra by
  Means of Frequency Modulation*. Journal of the Audio Engineering Society
  21(7): 526–534. El paper original. Disponible online.
- **Sound on Sound — Synth Secrets** partes 12, 13, 14, 15: introducciones
  pedagógicas a FM accesibles sin matemáticas.
- **DX7S Owner's Manual** (PDF en `_docs.md/DX7S.pdf`): la fuente oficial
  con todas las tablas DX7 ROM.
- **Dexed** (open source): emulador DX7 maduro, gran referencia para
  comparar resultados.

---

## Glosario rápido

| Término | Significado |
|---|---|
| **Operador** | Oscilador senoidal con su propio envelope. Bloque elemental DX7. |
| **Carrier** | Operador cuya salida llega al audio (su nivel = volumen). |
| **Modulator** | Operador que modula la frecuencia de otro (su nivel = brillo). |
| **Algoritmo** | Topología de cableado entre los 6 operadores. 32 disponibles. |
| **Ratio** | Frecuencia del operador como múltiplo de la nota pulsada. |
| **Modulation index** | "Cantidad de FM" — cuántos armónicos aparecen. |
| **Sideband** | Frecuencia adicional creada por modulación: `fc ± n·fm`. |
| **Feedback** | El operador se modula a sí mismo. Convierte seno en sierra/ruido. |
| **PMS** | Pitch Mod Sensitivity — cuánto el Mod Wheel ajusta la profundidad del LFO de pitch. |
| **AMS** | Amp Mod Sensitivity — cuánto el LFO de amplitud afecta al operador. |
| **EG Bias** | Atenuación estática controlada por Mod Wheel sobre operadores con AMS > 0. |
| **L4 = reposo** | Convención DX7: el envelope arranca y termina en L4. |
| **Level 50 (Pitch EG)** | Sin offset de pitch — convención DX7 ROM. |
