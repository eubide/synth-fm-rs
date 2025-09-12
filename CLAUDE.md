# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build and Run
```bash
cargo build --release          # Build optimized version
cargo run --release           # Run the DX7 emulator
cargo build --all-targets     # Build all targets including tests
RUST_LOG=debug cargo run      # Run with debug logging
```

### Code Quality
```bash
cargo fmt                     # Format code
cargo clippy                  # Run linter
cargo clippy -- -D warnings   # Fail on warnings
```

### Testing and Development
```bash
cargo check                   # Quick syntax check
cargo build --all-features    # Build with all features
```

## Architecture Overview

This is a Yamaha DX7 FM synthesizer emulator built with Rust, using a thread-safe architecture with shared state between audio and GUI threads.

### Core Components

**main.rs** - Application entry point that initializes the audio engine, MIDI handler, and GUI with shared synthesizer state via `Arc<Mutex<FmSynthesizer>>`.

**FmSynthesizer** (`fm_synth.rs`) - Central synthesizer engine managing:
- 16-voice polyphony with voice stealing
- Algorithm selection and routing
- Global parameters (master tune, mono/poly mode, portamento)
- Note on/off handling and voice allocation

**AudioEngine** (`audio_engine.rs`) - Real-time audio processing using CPAL:
- 44.1kHz sample rate with adaptive buffer sizing
- Lock-free audio thread communication
- Cross-platform audio backend abstraction

**Algorithm System** (`algorithms.rs`) - FM algorithm implementation:
- Loads 32 authentic DX7 algorithms from `algorithms.json`
- Column-centric graph layout for visual representation
- Comprehensive validation system for algorithm integrity
- Feedback loop detection and handling

**GUI System** (`gui.rs`) - egui-based DX7 interface emulation:
- Four operation modes: VOICE, ALGORITHM, OPERATOR, FUNCTION
- Real-time parameter control with immediate audio feedback
- Algorithm diagram visualization with automatic layout
- Preset management and selection

### Key Architectural Patterns

**Shared State Management**: The synthesizer state is wrapped in `Arc<Mutex<>>` and shared between:
- GUI thread (parameter updates)
- Audio thread (real-time processing)
- MIDI thread (note events)

**Algorithm Processing**: Uses recursive operator processing with feedback handling, where each algorithm defines carrier/modulator relationships loaded from JSON configuration.

**Voice Management**: Implements authentic DX7 voice allocation with:
- Polyphonic mode: Up to 16 simultaneous voices
- Monophonic mode: Single voice with portamento
- Intelligent voice stealing based on release times

### Configuration Files

**algorithms.json** - Defines all 32 DX7 algorithms with carrier/modulator connections and feedback loops. Critical for authentic FM synthesis behavior.

### Threading Model

The application uses three main threads:
1. **Main/GUI Thread**: egui interface and user interaction
2. **Audio Thread**: Real-time synthesis processing (CPAL callback)
3. **MIDI Thread**: MIDI input handling and event processing

State synchronization is handled through the shared `FmSynthesizer` instance with minimal lock contention.

### Algorithm Graph Layout System

Visual algorithm diagrams use a grid-based layout with these core principles:

#### Basic Layout Rules
- **Carriers**: Bottom row (row 0) 
- **Modulators**: Higher rows (1, 2, 3...) based on distance from carriers
- **No overlap**: Each operator gets unique grid position
- **Canvas**: 400x280px with automatic centering

#### Positioning Strategy
- **Layer assignment**: Modulators placed by shortest path to any carrier
- **Adjacent placement**: Connected operators positioned close to each other.
- **For connecteded operators**: above position is preferred, otherwise the position is occupied.
- **Conflict resolution**: Larger operator groups get priority in positioning
- **Canvas bounds**: All operators stay within visible area

#### Visual Specifications
- **Node size**: 20px circles
- **Spacing**: 35px vertical, 45px horizontal between operators
- **Connection lines**: Always visible, never hidden behind nodes
- **Feedback loops**: Small loops attached to operators for self-feedback 