# Guía de Usuario - Yamaha DX7 Emulator

## 🎹 Introducción a la Síntesis FM

El Yamaha DX7 utiliza **síntesis FM (Frecuencia Modulada)**, diferente a los sintetizadores analógicos tradicionales. En lugar de filtrar ondas (síntesis sustractiva), el DX7 usa **6 operadores** que se modulan entre sí para crear sonidos complejos.

## 🎮 Modos de Interfaz

El emulador tiene 4 modos principales, igual que el DX7 original:

### 🎵 VOICE Mode
- **Función**: Selección y carga de presets
- **Controles**: Grid de presets con botones clickeables
- **Uso**: Elige entre E.Piano, Bass, Brass, Strings, etc.

### ⚙️ ALGORITHM Mode  
- **Función**: Configuración del algoritmo FM y volumen maestro
- **Controles**: ComboBox con 32 algoritmos + slider de volumen
- **Uso**: Selecciona cómo se conectan los 6 operadores

### 🔧 OPERATOR Mode
- **Función**: Edición detallada de operadores individuales  
- **Controles**: Botones 1-6 + controles por operador + envolventes
- **Uso**: Ajusta frequency ratio, levels, detune, feedback, envolventes

### ⚡ FUNCTION Mode
- **Función**: Parámetros globales del sintetizador
- **Controles**: Master Tune, Poly/Mono, Pitch Bend, Portamento, Voice Init
- **Uso**: Configuración general del instrumento

## 🎛️ Controles Principales

### 1. OPERADORES (1-6) - Solo en OPERATOR Mode
Los 6 operadores son los bloques básicos del sonido. Cada uno puede ser:
- **Portador**: Genera el sonido audible
- **Modulador**: Modifica el timbre de otro operador

#### Parámetros de cada Operador:

**Frequency Ratio** (0.5 - 15.0)
- Controla la frecuencia del operador relativa a la nota tocada
- **Ratio 1.0**: Frecuencia fundamental (misma nota)
- **Ratio 2.0**: Una octava arriba
- **Ratio 0.5**: Una octava abajo
- **Ratio 3.0, 5.0, 7.0**: Armónicos impares (sonidos metálicos)

**Output Level** (0 - 99)
- Volumen del operador
- En moduladores: controla la intensidad de modulación (brillo/timbre)
- En portadores: controla el volumen directo

**Detune** (-7 to +7)
- Desafina ligeramente el operador
- Útil para sonidos más gruesos y chorus natural

**Feedback** (0 - 7, solo Operador 6)
- El operador se modula a sí mismo
- **0**: Sin feedback (onda sinusoidal pura)
- **3-4**: Añade armónicos (sonido de sierra)
- **7**: Máximo feedback (ruido/distorsión)

### 2. ENVOLVENTES (Rate/Level)

Cada operador tiene una envolvente de 4 etapas que controla cómo evoluciona en el tiempo:

**Rates (R1-R4)**: Velocidad de cada etapa (0-99)
- **R1**: Velocidad de ataque (99 = instantáneo, 50 = medio)
- **R2**: Velocidad de primer decay
- **R3**: Velocidad de segundo decay
- **R4**: Velocidad de release

**Levels (L1-L4)**: Nivel de cada punto (0-99)
- **L1**: Nivel máximo del ataque
- **L2**: Nivel después del primer decay
- **L3**: Nivel de sustain
- **L4**: Nivel final (normalmente 0)

### 3. ALGORITMOS (1-32)

El algoritmo define cómo se conectan los 6 operadores:

#### Algoritmos Populares:

**Algoritmo 1: Stack Completo**
```
6→5→4→3→2→1
```
- Todos en serie
- Sonido muy brillante y complejo
- Bueno para: Campanas, sonidos metálicos

**Algoritmo 5: Dos Stacks Paralelos**
```
6→3→1
5→2→1
    4→1
```
- Tres caminos de modulación
- Versátil para pads y strings

**Algoritmo 32: Todos en Paralelo**
```
1 + 2 + 3 + 4 + 5 + 6
```
- Síntesis aditiva (como un órgano)
- Cada operador es audible
- Bueno para: Órganos, pads suaves

## 🎵 Ejemplos de Sonidos

### 1. Piano Eléctrico (DX7 E.Piano)
**Configuración:**
- **Algoritmo**: 5
- **Op1**: Ratio=1.0, Level=99 (portador principal)
- **Op2**: Ratio=1.0, Level=85 (segundo portador)
- **Op3**: Ratio=14.0, Level=45 (modulador - brillo)
- **Op4**: Ratio=1.0, Level=50 (tercer portador)
- **Op5**: Ratio=1.0, Level=60 (modulador suave)
- **Op6**: Ratio=1.0, Level=70, Feedback=3

**Envolventes:**
- Portadores: Ataque rápido (R1=95), decay medio
- Moduladores: Ataque instantáneo, decay rápido para "tine"

### 2. Bajo Sintetizado
**Configuración:**
- **Algoritmo**: 1 (stack completo)
- **Op1**: Ratio=0.5, Level=99
- **Op2-5**: Ratios=1.0, 2.0, 3.0, 4.0, Levels decrecientes
- **Op6**: Ratio=0.5, Feedback=2

**Resultado**: Bajo profundo con armónicos

### 3. Campanas (Bells)
**Configuración:**
- **Algoritmo**: 7
- **Ratios no armónicos**: 1.0, 3.5, 5.3, 7.1
- **Envolventes**: Ataque instantáneo, decay largo

**Resultado**: Sonido metálico de campana

## ⚡ Function Mode - Parámetros Globales

### 🎼 Master Tune (-150 a +150 cents)
- **Función**: Ajuste de afinación global del sintetizador
- **Uso**: Sincronizar con otros instrumentos o efectos especiales
- **Ejemplo**: +50 cents para sonido ligeramente agudo
- **Reset**: Botón "Reset Tune" vuelve a A440

### 🎹 Poly/Mono Mode
- **POLY**: Modo polifónico (hasta 16 voces simultáneas) - **Defecto**
- **MONO**: Modo monofónico (solo una nota a la vez)
- **Uso**: MONO para leads y bajos, POLY para acordes y pads
- **Característica**: En MONO, las nuevas notas interrumpen las anteriores

### 🎚️ Pitch Bend Range (0-12 semitonos)
- **Función**: Define el rango máximo del pitch bend wheel/controlador
- **Defecto**: 2 semitonos (como el DX7 original)
- **Máximo**: 12 semitonos (1 octava completa)
- **Uso**: Ajustar según tu estilo de interpretación

### 🎵 Portamento (Solo en MONO Mode)
- **Enable/Disable**: Checkbox para activar el portamento
- **Time** (0-99): Velocidad del deslizamiento entre notas
  - **0**: Cambio instantáneo
  - **50**: Portamento medio
  - **99**: Transición muy lenta
- **Uso**: Para leads expresivos y sonidos de sintetizador clásico
- **Restricción**: Solo funciona en modo MONO (auténtico al DX7)

### 🔄 Voice Initialize
- **Función**: Resetea el preset actual a los valores básicos del DX7
- **Comportamiento**:
  - Solo Operador 1 activo con level 99
  - Todos los demás operadores level 0
  - Algoritmo 1 (stack básico)
  - Envelope básico tipo organ
  - Preset name cambia a "Init Voice"
- **Uso**: Punto de partida limpio para crear sonidos desde cero

## 🎛️ Flujo de Trabajo con Function Mode

### Configuración Inicial:
1. **FUNCTION** → Ajustar Master Tune si es necesario
2. **FUNCTION** → Elegir POLY (acordes) o MONO (leads)
3. **VOICE** → Seleccionar un preset base
4. **ALGORITHM** → Ajustar algoritmo si es necesario
5. **OPERATOR** → Personalizar operadores y envolventes

### Para Interpretación en Vivo:
1. **FUNCTION** → Configurar Pitch Bend Range según tu controlador
2. **FUNCTION** → Activar Portamento en modo MONO para leads expresivos
3. Usar **Voice Initialize** para volver al sonido básico rápidamente

### 4. Brass (Metales)
**Configuración:**
- **Algoritmo**: 16
- **Op1-3**: Portadores con ratios 1.0, 2.0, 3.0
- **Op4-6**: Moduladores con niveles altos
- **Envolventes**: Ataque medio (R1=75)

### 5. Strings (Cuerdas)
**Configuración:**
- **Algoritmo**: 5 o 14
- **Todos los ratios cercanos a 1.0 (0.99, 1.0, 1.01)
- **Detune ligero en varios operadores
- **Envolventes**: Ataque lento (R1=50)

## 🎹 Controles del Teclado

### Teclado Musical:
```
Octava Baja:    Z S X D C V G B H N J M
                C C# D D# E F F# G G# A A# B

Octava Alta:    Q 2 W 3 E R 5 T 6 Y 7 U
                C C# D D# E F F# G G# A A# B
```

### Controles:
- **↑/↓**: Cambiar octava
- **Espacio**: Panic (detener todas las notas)

## 💡 Tips para Crear Sonidos

### Para Sonidos Brillantes:
1. Usa algoritmos con muchas conexiones en serie (1-4)
2. Aumenta el Output Level de los moduladores
3. Usa ratios altos (7.0, 11.0, 14.0)

### Para Sonidos Suaves:
1. Usa algoritmos con operadores en paralelo (24-32)
2. Reduce los niveles de modulación
3. Mantén ratios cercanos a números enteros

### Para Sonidos Evolutivos:
1. Configura diferentes velocidades de envolvente en cada operador
2. Los moduladores con decay rápido crean "attack" característico
3. Los portadores con release largo crean colas suaves

### Para Sonidos de Bajo:
1. Usa ratios de 0.5 o 1.0 en los portadores
2. Añade un poco de feedback (2-3) para calidez
3. Moduladores con ratios 2.0, 3.0 para armónicos

### Para Efectos Especiales:
1. Usa ratios no armónicos (1.41, 3.14, 5.67)
2. Feedback alto (5-7) para ruido
3. Envolventes muy rápidas o muy lentas

## 🔧 Flujo de Trabajo Recomendado

1. **Selecciona un Algoritmo** apropiado para tu sonido objetivo
2. **Configura los Portadores** (los que suenan directamente)
3. **Ajusta los Moduladores** para dar color al timbre
4. **Afina las Envolventes** para la evolución temporal
5. **Añade Detune** para amplitud estéreo
6. **Experimenta con Feedback** en Op6 para textura

## 📊 Tabla de Referencia Rápida

| Tipo de Sonido | Algoritmo | Ratios Típicos | Feedback | Function Mode | Característica |
|----------------|-----------|----------------|----------|---------------|----------------|
| Piano E. | 5, 6 | 1, 3.5, 7, 14 | 2-4 | POLY, Bend=2 | Attack metálico |
| Bajo | 1, 2 | 0.5, 1, 2 | 1-3 | MONO, Porta=20 | Fundamental fuerte |
| Pad | 14, 19 | 1, 1.01, 2 | 0-2 | POLY, Bend=5 | Evolución lenta |
| Lead | 8, 11 | 1, 2, 3, 5 | 3-5 | MONO, Porta=40, Bend=7 | Brillante, cortante |
| Bells | 7, 9 | 1, 3.5, 5.3 | 0-1 | POLY, Bend=2 | Inarmónico |
| Brass | 16, 22 | 1, 2, 3, 4 | 2-4 | MONO, Bend=5 | Attack medio |
| Organ | 32 | 1, 2, 3, 4, 5, 6 | 0 | POLY, Bend=0 | Aditivo puro |

### Leyenda Function Mode:
- **POLY/MONO**: Modo recomendado
- **Porta**: Portamento Time (solo en MONO)
- **Bend**: Pitch Bend Range sugerido

## 🎯 Ejercicios Prácticos

### Ejercicio 1: Crear un Piano Eléctrico
1. Selecciona Algoritmo 5
2. Op1: Ratio=1, Level=99
3. Op3: Ratio=7, Level=35 (da el "tine")
4. Op6: Feedback=3
5. Ajusta envolventes para attack rápido

### Ejercicio 2: Diseñar un Bajo Profundo
1. Algoritmo 1
2. Op1: Ratio=0.5, Level=99
3. Op2-6: Ratios incrementales, levels decrecientes
4. R1=99 en todos para attack punchy

### Ejercicio 3: Pad Atmosférico
1. Algoritmo 14
2. Todos los ratios cerca de 1.0
3. Detune variado (+3, -2, +1, etc.)
4. Envolventes lentas (R1=30-50)

Recuerda: ¡La síntesis FM es experimental! No hay configuraciones "incorrectas", solo diferentes timbres por descubrir.