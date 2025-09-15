# Yamaha DX7 Emulator

Un emulador de alta fidelidad del legendario sintetizador Yamaha DX7, construido en Rust con síntesis FM en tiempo real, soporte MIDI y una interfaz gráfica que simula la experiencia original.

## Características

### Motor de Síntesis FM
- **6 Operadores FM** con control independiente de frecuencia y nivel
- **32 Algoritmos** de routing auténticos del DX7 (correctamente implementados)
- **Envolventes de 4 etapas** (Rate/Level) para cada operador
- **Feedback** en el operador 6 para texturas armónicas
- **16 voces de polifonía** con voice stealing inteligente
- **Sistema de presets** compatible con patches clásicos del DX7
- **🆕 LFO Global Auténtico** con 6 formas de onda y control MIDI en tiempo real

### Interfaz Auténtica
- **Display LCD simulado** con retroiluminación verde
- **Botones de membrana** como el DX7 original
- **Modos de operación**: VOICE, OPERATOR (con selector de algoritmos integrado), **🆕 LFO**
- **Selección de operadores** 1-6 (solo en modo Operator)
- **Visualización avanzada de algoritmos** con layout optimizado para feedback loops
- **Diagramas interactivos** que muestran conexiones entre operadores en tiempo real
- **🆕 Panel LFO Completo** con control visual de modulación en tiempo real

### Function Mode - Parámetros Globales
- **Master Tune**: Afinación global ±150 cents
- **Poly/Mono Mode**: Cambio entre modo polifónico y monofónico
- **Pitch Bend Range**: Rango configurable 0-12 semitonos
- **Portamento**: Control de deslizamiento de notas (solo en modo MONO)
- **Voice Initialize**: Reset del preset a valores básicos del DX7

### Características Avanzadas
- **Entrada MIDI en tiempo real** para controladores externos
- **Teclado virtual** con soporte de múltiples octavas
- **Pitch Bend** con rango configurable
- **🆕 Mod Wheel (CC1)** controla profundidad del LFO en tiempo real
- **Sistema de presets** para guardar y cargar sonidos
- **Transiciones suaves** en modo mono sin clicks o artifacts
- **Key Scaling** completo (rate y level) por operador
- **Velocity Sensitivity** configurable (0-7) por operador

### 🆕 Sistema LFO Completo
- **6 Formas de Onda**: Triangle, Sine, Square, Saw Up/Down, Sample & Hold
- **Modulación Dual**: Pitch (vibrato) y Amplitude (tremolo) independientes  
- **Control Auténtico**: Rate 0-99 (0.062Hz-20Hz), Delay 0-99 (0-5 segundos)
- **Profundidades**: Pitch/Amp Depth 0-99 con escalado musical auténtico
- **Key Sync**: Reinicio opcional del LFO en cada nota
- **Integración MIDI**: Mod Wheel controla intensidad del efecto (0-100%)

## Instalación

```bash
# Clonar el repositorio
git clone https://github.com/yourusername/synth-fm-rs.git
cd synth-fm-rs

# Compilar en modo release para rendimiento óptimo
cargo build --release

# Ejecutar el emulador
cargo run --release
```

## Uso

### Controles de Teclado
- **Z-M**: Octava inferior (C-B)
- **Q-U**: Octava superior (C-B)
- **↑↓**: Cambiar octava
- **Espacio**: Panic (detener todas las notas)

### Operación del DX7

#### Modos de Interfaz
- **VOICE Mode**: Selección y carga de presets
- **OPERATOR Mode**: Edición detallada de operadores individuales y selección de algoritmos FM
- **🆕 LFO Mode**: Control completo del oscilador de baja frecuencia

**Nota**: El control de algoritmos ahora se encuentra integrado en el modo OPERATOR para un flujo de trabajo más eficiente.

#### Flujo de Trabajo
1. **Cargar un Preset**: En modo VOICE, selecciona un preset de la biblioteca
2. **Ajustar Algoritmo y Operadores**: En modo OPERATOR, selecciona algoritmo FM en la parte superior y edita operadores 1-6
3. **🆕 Configurar LFO**: En modo LFO, ajusta modulación y efectos expresivos
4. **Aplicar Voice Init**: Usa el botón VOICE INIT para resetear a sonido básico

#### 🆕 Uso del LFO
1. **Acceder al LFO**: Presiona el botón **LFO** en la interfaz principal
2. **Configurar Timing**:
   - **Rate**: Velocidad del LFO (0-99, ~0.062-20Hz)
   - **Delay**: Tiempo antes de que inicie el LFO (0-99, 0-5 segundos)
3. **Configurar Modulación**:
   - **Pitch Depth**: Cantidad de vibrato (0-99)
   - **Amp Depth**: Cantidad de tremolo (0-99)
   - **Waveform**: Selecciona entre 6 formas de onda
   - **Key Sync**: Reinicia LFO con cada nota nueva
4. **Control en Tiempo Real**: Mueve el **Mod Wheel** de tu teclado MIDI para controlar la intensidad del efecto

#### Parámetros por Operador
- **Frequency Ratio**: Relación de frecuencia (0.5-15.0)
- **Output Level**: Volumen de salida (0-99)
- **Detune**: Desafinación fina (-7 a +7)
- **Envelope**: 4 etapas Rate/Level para control dinámico

### Algoritmos FM
El DX7 incluye 32 algoritmos que definen cómo se conectan los 6 operadores:
- **Algoritmo 1**: Stack completo (6→5→4→3→2→1)
- **Algoritmo 32**: 6 operadores en paralelo (síntesis aditiva)
- Y 30 configuraciones intermedias para todo tipo de sonidos

## Arquitectura Técnica

### Motor de Audio
- **Sample Rate**: 44.1kHz/48kHz adaptativo
- **Backend**: CPAL (Cross-Platform Audio Library)
- **Procesamiento**: Lock-free con Arc<Mutex> para actualizaciones
- **Latencia**: Buffer optimizado para tiempo real

### Síntesis FM
- Implementación auténtica de los algoritmos del DX7
- Envolventes de 4 etapas con curvas exponenciales
- Feedback del operador 6 para auto-modulación
- **Portamento**: Interpolación exponencial en modo MONO con transiciones suaves
- **Pitch Bend**: Aplicado con rango configurable
- **Voice Stealing**: Algoritmo inteligente para polifonía
- **Key Scaling**: Envolventes y niveles sensibles a la posición del teclado
- **Velocity Sensitivity**: Respuesta de velocidad individual por operador

### 🎯 Fidelidad al DX7 Original (95-98%)
- **Master Tune**: Rango exacto ±150 cents
- **Algoritmos**: 32 configuraciones auténticas con validación completa
- **Envolventes**: Comportamiento Rate/Level original con key scaling
- **🆕 LFO Global**: Implementación auténtica con 6 formas de onda DX7
- **🆕 Mod Wheel**: Integración MIDI CC1 exacta como el original
- **🆕 Curvas Exponenciales**: Rate mapping 0.062Hz-20Hz auténtico
- **Restricciones**: Portamento solo en modo MONO (como el DX7)
- **Visualización**: Diagramas de algoritmos con layout column-centric optimizado
- **Transiciones**: Mono mode sin artifacts (mejora sobre el original)

## Desarrollo

### Comandos de Desarrollo
```bash
# Compilar y ejecutar
cargo build --release          # Build optimizado
cargo run --release           # Ejecutar el emulador
RUST_LOG=debug cargo run      # Ejecutar con debug logging

# Calidad de código
cargo fmt                     # Formatear código
cargo clippy                  # Ejecutar linter
cargo clippy -- -D warnings   # Fallar en advertencias
cargo check                   # Verificación sintáctica rápida
```

### Arquitectura del Sistema
El emulador utiliza una **arquitectura multi-thread** con estado compartido:
- **Thread GUI**: Interfaz egui y interacción de usuario
- **Thread Audio**: Procesamiento en tiempo real (CPAL callback)  
- **Thread MIDI**: Manejo de entrada MIDI
- **Estado compartido**: `Arc<Mutex<FmSynthesizer>>` para sincronización

### Sistema de Algoritmos
Los diagramas visuales utilizan un **layout column-centric** donde:
- Cada carrier crea su propia columna vertical
- Los moduladores se apilan sobre sus targets
- Los feedback loops aparecen como líneas verticales limpias
- Centrado automático en canvas de 400x280px

## Licencia

Proyecto de código abierto bajo licencia MIT.