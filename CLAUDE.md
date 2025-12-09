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

This is a DX7-style FM synthesizer emulator built with Rust, using a **lock-free architecture** with message queues and state snapshots for zero-contention communication between audio and GUI threads.

### Core Components

**main.rs** - Application entry point that initializes the audio engine, MIDI handler, and GUI. Creates `SynthEngine` (audio thread) and `SynthController` (GUI/MIDI threads) via `create_synth()`.

**fm_synth.rs** - Central synthesizer module with two main components:
- **SynthEngine**: Runs on audio thread, processes commands, generates audio
- **SynthController**: Interface for GUI/MIDI threads, sends commands via ringbuffer
- 16-voice polyphony with voice stealing
- Algorithm selection and routing
- Global parameters (master tune, mono/poly mode, portamento)

**AudioEngine** (`audio_engine.rs`) - Real-time audio processing using CPAL:
- 44.1kHz sample rate with adaptive buffer sizing
- Processes commands from ringbuffer at start of each buffer
- Publishes state snapshots for GUI consumption
- Cross-platform audio backend abstraction

**Algorithm System** (`algorithms.rs`) - FM algorithm implementation:
- 32 authentic DX7 algorithms hardcoded for performance
- Column-centric graph layout for visual representation
- Comprehensive validation system for algorithm integrity
- Feedback loop detection and handling

**GUI System** (`gui.rs`) - egui-based DX7 interface emulation:
- Four operation modes: VOICE, OPERATOR, LFO, EFFECTS
- Real-time parameter control via command queue (lock-free)
- Reads state from snapshots (never blocks audio thread)
- Algorithm diagram visualization with automatic layout
- Preset management and selection

### Key Architectural Patterns

**Lock-Free Communication**: Zero-contention message passing between threads:
- **CommandQueue** (`command_queue.rs`): SPSC ringbuffer (GUI/MIDI -> Audio)
- **StateSnapshot** (`state_snapshot.rs`): Triple buffer with atomic swap (Audio -> GUI)
- GUI reads snapshots for display, sends commands for changes
- Audio thread processes commands at buffer start, publishes snapshot at end

**Algorithm Processing**: Uses recursive operator processing with feedback handling, where each algorithm defines carrier/modulator relationships hardcoded in Rust for optimal performance.

**Voice Management**: Implements authentic DX7 voice allocation with:
- Polyphonic mode: Up to 16 simultaneous voices
- Monophonic mode: Single voice with portamento
- Intelligent voice stealing based on note-on order (oldest voice first)

### Threading Model

The application uses three main threads with lock-free communication:
1. **Main/GUI Thread**: egui interface, reads StateSnapshot, sends SynthCommands
2. **Audio Thread**: Real-time synthesis via SynthEngine (owns all audio state)
3. **MIDI Thread**: MIDI input, sends SynthCommands via SynthController

No mutexes in the audio path. GUI and MIDI threads never block audio processing.

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