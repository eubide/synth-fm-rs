# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2025-09-12

### Added
- **Column-Centric Algorithm Layout System**: Complete rewrite of algorithm visualization
  - Each carrier operator creates its own vertical column
  - Modulators stack vertically above their target operators
  - Feedback loops render as clean vertical lines within columns
  - Automatic centering within 400x280px canvas
- **Enhanced Graph Layout Algorithm**: Optimized spacing and positioning
  - Reduced spacing: margin 30px→15px, layer 50px→35px, column 80px→45px
  - Improved horizontal and vertical centering
  - Operator size reduced to 20px for better fit
- **Comprehensive Architecture Documentation**: Added CLAUDE.md with development guidelines

### Fixed
- **Mono Mode Glitch**: Eliminated clicking/popping when switching notes in monophonic mode
  - Changed from abrupt `stop()` to smooth `release()` transition
  - Maintains portamento functionality while removing audio artifacts
- **Algorithm Graph Positioning**: Fixed feedback loop visualization
  - Operators in feedback relationships now share same X coordinate
  - Clean vertical lines for feedback connections
  - No more overlapping or intersecting connection lines

### Changed  
- **Algorithm Layout Architecture**: Migrated from layer-centric to column-centric approach
  - Simplified code from ~180 lines to ~135 lines
  - Modular functions: `build_columns()`, `assign_columns()`, `find_column_position()`
  - Improved maintainability and performance
- **Graph Rendering Performance**: Optimized positioning calculations
  - Single-pass column construction
  - Eliminated recursive complexity
  - Reduced computational overhead for real-time updates

### Technical Details
- **Threading Model**: Multi-thread architecture with shared state via `Arc<Mutex<FmSynthesizer>>`
- **Audio Engine**: Real-time processing with CPAL backend at 44.1kHz
- **Algorithm System**: JSON-based configuration with comprehensive validation
- **GUI Framework**: egui-based authentic DX7 interface emulation

### Performance Improvements
- Simplified layout algorithm reduces CPU usage during algorithm switching
- Optimized graph positioning for smoother UI interactions
- Reduced memory allocations in visual rendering pipeline

---

## Project Status
- **Fidelity**: 85-90% authentic to original Yamaha DX7
- **Features**: 32 algorithms, 16-voice polyphony, key scaling, velocity sensitivity
- **Improvements over original**: Smooth mono transitions, enhanced visualization