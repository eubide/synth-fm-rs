# 🎛️ Algorithm Design Guide - DX7 FM Synthesizer

## Overview

This guide explains how to design custom algorithms using the new **matrix-based system** introduced in 2025-09-15. The system replaces the complex JSON-based approach with a simple, intuitive 6x6 modulation matrix.

## Core Concepts

### The Modulation Matrix

Each algorithm is represented as a **6x6 matrix** where:
- **Rows**: Source operators (modulators)
- **Columns**: Target operators (modulated)
- **Values**: Modulation amount (0.0 = no modulation, 1.0 = full modulation)

```
        Op1  Op2  Op3  Op4  Op5  Op6  (to)
Op1   [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
Op2   [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]  ← Op2 modulates Op1
Op3   [0.0, 1.0, 0.0, 0.0, 0.0, 0.0]  ← Op3 modulates Op2
Op4   [0.0, 0.0, 1.0, 0.0, 0.0, 0.0]
Op5   [0.0, 0.0, 0.0, 1.0, 0.0, 0.0]
Op6   [0.0, 0.0, 0.0, 0.0, 1.0, 0.7]  ← Op6 self-feedback
(from)
```

### Carriers vs Modulators

- **Carriers**: Operators that output to audio (sound you hear)
- **Modulators**: Operators that only modulate other operators

## Quick Start Guide

### Method 1: Using the Vec! Format

Add your algorithm to `algorithm_migration.rs`:

```rust
let algorithms = vec![
    // ... existing algorithms ...
    (36, "My Custom", vec![1], vec![(2,1), (3,2), (4,3)]),
];
```

**Format**: `(number, "name", carriers, connections)`

- `number`: Unique ID (36, 37, 38...)
- `"name"`: Descriptive name
- `carriers`: Which operators output to audio `vec![1, 3, 5]`
- `connections`: Modulation routes `vec![(from, to), ...]`

### Method 2: Using the Helper Function

```rust
use crate::algorithm_matrix::create_custom_algorithm;

let my_algorithm = create_custom_algorithm(
    36,                               // Algorithm number
    "Bell Synthesis",                 // Name
    &[1, 2],                         // Carriers: Op1 and Op2 to audio
    &[(3,1), (4,2), (5,3), (6,4)],   // Chain: Op6→Op5→Op4→Op3→Op2→Op1
    &[(5, 0.8), (6, 0.5)]            // Custom feedback: Op5=80%, Op6=50%
);
```

## Algorithm Patterns

### 1. Serial Stacks

**Concept**: Operators modulate in a chain
```rust
// Simple 4-operator stack: Op4 → Op3 → Op2 → Op1 → Audio
(37, "4-Op Stack", vec![1], vec![(2,1), (3,2), (4,3)])
```

**Sound**: Rich harmonic content, complex timbres

### 2. Parallel Carriers

**Concept**: Multiple independent outputs
```rust
// Three separate carriers
(38, "Triple Wide", vec![1, 3, 5], vec![(2,1), (4,3), (6,5)])
```

**Sound**: Wide, thick textures

### 3. Feedback Algorithms

**Concept**: Self-modulation creates complex harmonics
```rust
// Stack with feedback
(39, "Stack + FB", vec![1], vec![(2,1), (3,2), (4,3), (4,4)])
```

**Sound**: Aggressive, distorted, bell-like

### 4. Cross-Modulation

**Concept**: Operators modulate multiple targets
```rust
// Op3 modulates both Op1 and Op2
(40, "Cross Mod", vec![1, 2], vec![(3,1), (3,2), (4,3)])
```

**Sound**: Complex interactions, metallic tones

### 5. Ring Modulation

**Concept**: Two oscillators modulating each other
```rust
// Op1 and Op2 cross-modulate
(41, "Ring Mod", vec![1], vec![(1,2), (2,1), (3,1)])
```

**Sound**: Inharmonic, clangorous, metallic

## Design Tips

### Carrier Strategies

- **1 Carrier**: Focused, monophonic-style sounds
- **2 Carriers**: Dual-timbre effects, detuned sounds  
- **3+ Carriers**: Wide, organ-like textures
- **6 Carriers**: Maximum width, additive synthesis

### Feedback Guidelines

- **0.1-0.3**: Subtle harmonic enhancement
- **0.4-0.6**: Noticeable timbre change
- **0.7-0.8**: Strong character, bell-like
- **0.9-1.0**: Aggressive distortion, noise

### Common Modulation Amounts

```rust
// Standard amounts used in DX7
0.0  // No modulation
0.5  // Light modulation
0.7  // Self-feedback (moderate)
1.0  // Full modulation
```

## Example Algorithms

### Electric Piano
```rust
(42, "E.Piano Classic", vec![1, 3, 5], vec![
    (2,1),   // Op2 → Op1 (sine + modulation)
    (4,3),   // Op4 → Op3 (harmonic)
    (6,5),   // Op6 → Op5 (bell-like)
    (6,6)    // Op6 self-feedback
])
```

### Brass Section
```rust
(43, "Brass Stack", vec![1], vec![
    (2,1), (3,2), (4,3),  // Main stack
    (5,2),                // Cross-modulation for buzz
    (3,3)                 // Light feedback
])
```

### Percussive Bell
```rust
(44, "Tubular Bell", vec![1, 2], vec![
    (3,1), (4,2),         // Parallel modulators
    (5,3), (6,4),         // Higher harmonics
    (5,5), (6,6)          // Strong feedback
])
```

### Bass Synth
```rust
(45, "Sub Bass", vec![1], vec![
    (2,1),                // Basic modulation
    (3,1),                // Harmonics
    (4,2), (5,4),         // Complex chain
    (2,2)                 // Self-feedback
])
```

## Advanced Techniques

### Asymmetric Feedback
```rust
// Different feedback amounts for different operators
&[(2, 0.3), (4, 0.8), (6, 0.5)]
```

### Feedback Loops
```rust
// Operators feeding each other (not just self-feedback)
vec![(1,2), (2,3), (3,1)]  // Triangle feedback loop
```

### Harmonic Series
```rust
// Ratios designed for harmonic stacking
// Op ratios: 1.0, 2.0, 3.0, 4.0, 5.0, 6.0
(46, "Harmonic Stack", vec![1], vec![
    (2,1), (3,1), (4,1), (5,1), (6,1)  // All harmonics to fundamental
])
```

## Testing Your Algorithms

### 1. Add to Migration File
```rust
// In algorithm_migration.rs
(your_number, "Your Name", vec![carriers], vec![connections])
```

### 2. Update GUI Range
```rust
// In gui.rs, change the loop range:
for i in 1..=45 {  // Update to your highest algorithm number
```

### 3. Compile and Test
```bash
cargo build --release
cargo run --release
```

### 4. Test Different Presets
- Try your algorithm with different operator settings
- Test with various envelope shapes
- Experiment with LFO modulation

## Sound Design Tips

### For Bells and Mallets
- Use feedback (0.5-0.8)
- Multiple carriers (2-3)
- Shorter modulation chains

### For Brass and Leads  
- Longer stacks (4-6 operators)
- Single carrier
- Moderate feedback (0.3-0.5)

### For Pads and Strings
- Multiple carriers (3-6)
- Parallel structures
- Light feedback (0.1-0.3)

### For Bass and Percussion
- Single carrier
- Strong low-frequency modulation
- Higher feedback for aggression

## Troubleshooting

### Algorithm Not Appearing
- Check algorithm number is unique
- Verify GUI range includes your number
- Ensure proper syntax in Vec!

### No Sound
- Verify at least one carrier is set
- Check connections are valid (1-6 range)
- Ensure operators have proper levels

### Unexpected Sound
- Check operator ratios and levels
- Verify envelope settings
- Test with simpler algorithm first

## Future Enhancements

The matrix system enables several exciting possibilities:

1. **Real-time Matrix Editor**: Visual 6x6 grid for live editing
2. **Morphing Algorithms**: Interpolate between different matrices
3. **User Presets**: Save/load custom algorithm collections
4. **Probability Matrices**: Randomized connections for generative music
5. **MIDI Control**: Map matrix values to MIDI controllers

---

Happy algorithm designing! The matrix system gives you the power to create any FM synthesis configuration imaginable, from authentic DX7 recreations to entirely new sonic territories.