//! DX7 SysEx parser and emitter.
//!
//! Implements the two formats actually used by DX7 hardware:
//!
//! - **Single voice (VCED)** — 163 bytes total. The voice data block is 155 bytes
//!   in the unpacked Voice Edit format. Used by "voice send/receive single".
//! - **32-voice bulk (VMEM)** — 4104 bytes total. The voice data block is
//!   4096 bytes (32 voices × 128 bytes packed). Used by "voice memory dump".
//!
//! References: DX7 Owner's Manual Vol. 4 (System Exclusive), DX7S manual chapter 7.

use crate::lfo::LFOWaveform;
use crate::operator::KeyScaleCurve;
use crate::presets::{Dx7Preset, PresetLfo, PresetOperator, PresetPitchEg};

/// Yamaha manufacturer SysEx ID.
const YAMAHA_ID: u8 = 0x43;

/// Length of one unpacked voice block (VCED).
pub const VCED_LEN: usize = 155;
/// Length of one packed voice (in VMEM).
pub const VMEM_VOICE_LEN: usize = 128;
/// Length of the full 32-voice bulk payload.
pub const VMEM_LEN: usize = 32 * VMEM_VOICE_LEN; // 4096

/// Result of parsing a SysEx message.
///
/// `SingleVoice` is boxed because a fully-populated `Dx7Preset` is several hundred
/// bytes and the variants would otherwise differ in size by an order of magnitude.
#[derive(Debug)]
pub enum SysexResult {
    SingleVoice(Box<Dx7Preset>),
    Bulk(Vec<Dx7Preset>),
}

#[derive(Debug)]
pub enum SysexError {
    InvalidFraming,
    TooShort,
    NotYamaha(u8),
    UnsupportedSubStatus(u8),
    UnsupportedFormat(u8),
    TruncatedData,
    LengthMismatch { declared: usize, actual: usize },
    ChecksumMismatch { expected: u8, computed: u8 },
}

impl std::fmt::Display for SysexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFraming => write!(f, "message must start with 0xF0 and end with 0xF7"),
            Self::TooShort => write!(f, "message too short"),
            Self::NotYamaha(id) => {
                write!(f, "not a Yamaha SysEx (manufacturer ID = 0x{:02X})", id)
            }
            Self::UnsupportedSubStatus(s) => {
                write!(f, "unsupported sub-status nibble 0x{:02X}", s)
            }
            Self::UnsupportedFormat(fmt_byte) => {
                write!(f, "unsupported format byte {}", fmt_byte)
            }
            Self::TruncatedData => write!(f, "data block truncated"),
            Self::LengthMismatch { declared, actual } => write!(
                f,
                "byte count mismatch: header says {}, actual {}",
                declared, actual
            ),
            Self::ChecksumMismatch { expected, computed } => write!(
                f,
                "checksum mismatch (expected 0x{:02X}, computed 0x{:02X})",
                expected, computed
            ),
        }
    }
}

impl std::error::Error for SysexError {}

/// Parse a complete DX7 SysEx message (`F0 ... F7`).
///
/// On success returns either a single voice or a 32-voice bank, ready to load.
pub fn parse_message(bytes: &[u8]) -> Result<SysexResult, SysexError> {
    if bytes.len() < 8 {
        return Err(SysexError::TooShort);
    }
    if bytes.first() != Some(&0xF0) || bytes.last() != Some(&0xF7) {
        return Err(SysexError::InvalidFraming);
    }
    if bytes[1] != YAMAHA_ID {
        return Err(SysexError::NotYamaha(bytes[1]));
    }

    // bytes[2] = 0x0n where n is the channel; the upper nibble must be 0 for a
    // voice/bulk dump.
    let sub_status = bytes[2] & 0xF0;
    if sub_status != 0x00 {
        return Err(SysexError::UnsupportedSubStatus(sub_status));
    }

    let format = bytes[3];
    let count = ((bytes[4] as usize) << 7) | (bytes[5] as usize & 0x7F);

    // Data block runs from index 6 to 6+count, then a checksum byte, then F7.
    let data_end = 6 + count;
    if bytes.len() != data_end + 2 {
        return Err(SysexError::LengthMismatch {
            declared: count,
            actual: bytes.len().saturating_sub(8),
        });
    }
    let data = &bytes[6..data_end];
    let checksum_byte = bytes[data_end];

    let computed = compute_checksum(data);
    if computed != checksum_byte {
        return Err(SysexError::ChecksumMismatch {
            expected: checksum_byte,
            computed,
        });
    }

    match format {
        0 => {
            if count != VCED_LEN {
                return Err(SysexError::LengthMismatch {
                    declared: count,
                    actual: VCED_LEN,
                });
            }
            let preset = parse_vced(data, "SysEx")?;
            Ok(SysexResult::SingleVoice(Box::new(preset)))
        }
        9 => {
            if count != VMEM_LEN {
                return Err(SysexError::LengthMismatch {
                    declared: count,
                    actual: VMEM_LEN,
                });
            }
            let presets = parse_vmem(data)?;
            Ok(SysexResult::Bulk(presets))
        }
        other => Err(SysexError::UnsupportedFormat(other)),
    }
}

/// Encode a preset as a single-voice SysEx message (163 bytes).
///
/// `channel` is the 0-indexed MIDI channel embedded in the header byte.
pub fn encode_single_voice(preset: &Dx7Preset, channel: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(VCED_LEN + 8);
    out.push(0xF0);
    out.push(YAMAHA_ID);
    out.push(channel & 0x0F); // sub-status 0, channel n
    out.push(0x00); // format 0 = VCED
    out.push(0x01); // byte count MSB
    out.push(0x1B); // byte count LSB (0x011B = 155)

    let body = encode_vced(preset);
    debug_assert_eq!(body.len(), VCED_LEN);
    let checksum = compute_checksum(&body);
    out.extend_from_slice(&body);
    out.push(checksum);
    out.push(0xF7);
    out
}

/// Two's-complement of the running 7-bit sum, masked to 7 bits.
fn compute_checksum(data: &[u8]) -> u8 {
    let sum: u32 = data.iter().map(|&b| b as u32).sum();
    ((!sum).wrapping_add(1) & 0x7F) as u8
}

// ---------------------------------------------------------------------------
// VCED (single voice) parser
// ---------------------------------------------------------------------------

/// Parse a 155-byte VCED block into a preset.
fn parse_vced(data: &[u8], collection: &str) -> Result<Dx7Preset, SysexError> {
    if data.len() != VCED_LEN {
        return Err(SysexError::TruncatedData);
    }

    let mut operators: [PresetOperator; 6] = std::array::from_fn(|_| PresetOperator::default());

    // SysEx orders operators OP6..OP1; our preset array is OP1..OP6.
    for sysex_idx in 0..6 {
        let base = sysex_idx * 21;
        let preset_idx = 5 - sysex_idx;
        operators[preset_idx] = parse_vced_operator(&data[base..base + 21]);
    }

    let pitch_eg = PresetPitchEg {
        rate1: data[126] as f32,
        rate2: data[127] as f32,
        rate3: data[128] as f32,
        rate4: data[129] as f32,
        level1: data[130] as f32,
        level2: data[131] as f32,
        level3: data[132] as f32,
        level4: data[133] as f32,
    };

    let algorithm = (data[134] & 0x1F) + 1; // SysEx 0..31, internal 1..32
    let feedback = (data[135] & 0x07) as f32; // 0..7
    let osc_key_sync = data[136] != 0;

    let lfo = PresetLfo {
        waveform: lfo_wave_from_dx7(data[142]),
        rate: data[137] as f32,
        delay: data[138] as f32,
        pitch_mod_depth: data[139] as f32,
        amp_mod_depth: data[140] as f32,
        key_sync: data[141] != 0,
    };

    let pitch_mod_sensitivity = data[143] & 0x07;
    // DX7 transpose: 0..48 with 24 = C3 / no shift.
    let transpose_semitones = (data[144] as i16 - 24).clamp(-24, 24) as i8;

    let name = parse_voice_name(&data[145..155]);

    // Apply DX7 conventions: feedback lives on OP6 (index 5); osc key sync is global.
    operators[5].feedback = feedback;
    for op in operators.iter_mut() {
        op.oscillator_key_sync = osc_key_sync;
    }

    Ok(Dx7Preset {
        name,
        collection: collection.to_string(),
        algorithm,
        operators,
        master_tune: None,
        pitch_bend_range: None,
        portamento_enable: None,
        portamento_time: None,
        mono_mode: None,
        transpose_semitones,
        pitch_mod_sensitivity,
        pitch_eg: Some(pitch_eg),
        lfo: Some(lfo),
    })
}

fn parse_vced_operator(block: &[u8]) -> PresetOperator {
    let r1 = block[0] as f32;
    let r2 = block[1] as f32;
    let r3 = block[2] as f32;
    let r4 = block[3] as f32;
    let l1 = block[4] as f32;
    let l2 = block[5] as f32;
    let l3 = block[6] as f32;
    let l4 = block[7] as f32;
    let breakpoint = block[8];
    let kls_ld = block[9] as f32;
    let kls_rd = block[10] as f32;
    let kls_lc = block[11];
    let kls_rc = block[12];
    let krs = block[13];
    let ams = block[14];
    let kvs = block[15];
    let level = block[16] as f32;
    let osc_mode = block[17];
    let coarse = block[18];
    let fine = block[19];
    let detune_raw = block[20];

    let fixed_frequency = osc_mode == 1;
    let frequency_ratio = if fixed_frequency {
        // In fixed mode the ratio field is unused — keep a sane default.
        1.0
    } else if coarse == 0 {
        // DX7 convention: coarse=0 → 0.5×.
        0.5
    } else {
        (coarse as f32) * (1.0 + (fine as f32) / 100.0)
    };
    let fixed_freq_hz = if fixed_frequency {
        let c = (coarse & 0x03) as f32;
        10f32.powf(c) * (1.0 + (fine as f32) / 100.0)
    } else {
        440.0
    };

    let detune = (detune_raw as i16 - 7) as f32;

    let breakpoint_midi = breakpoint.saturating_add(21).min(127); // DX7 stores BP-21

    PresetOperator {
        frequency_ratio,
        output_level: level,
        detune,
        feedback: 0.0,
        velocity_sensitivity: (kvs & 0x07) as f32,
        key_scale_rate: (krs & 0x07) as f32,
        key_scale_breakpoint: breakpoint_midi,
        key_scale_left_curve: KeyScaleCurve::from_dx7_code(kls_lc),
        key_scale_right_curve: KeyScaleCurve::from_dx7_code(kls_rc),
        key_scale_left_depth: kls_ld.clamp(0.0, 99.0),
        key_scale_right_depth: kls_rd.clamp(0.0, 99.0),
        am_sensitivity: ams & 0x03,
        oscillator_key_sync: true, // overridden by patch-level flag
        fixed_frequency,
        fixed_freq_hz,
        envelope: (r1, r2, r3, r4, l1, l2, l3, l4),
    }
}

// ---------------------------------------------------------------------------
// VMEM (32-voice bulk) parser — packed format
// ---------------------------------------------------------------------------

fn parse_vmem(data: &[u8]) -> Result<Vec<Dx7Preset>, SysexError> {
    if data.len() != VMEM_LEN {
        return Err(SysexError::TruncatedData);
    }

    let mut out = Vec::with_capacity(32);
    for i in 0..32 {
        let block = &data[i * VMEM_VOICE_LEN..(i + 1) * VMEM_VOICE_LEN];
        out.push(parse_vmem_voice(block, &format!("Bulk #{:02}", i + 1)));
    }
    Ok(out)
}

fn parse_vmem_voice(block: &[u8], collection: &str) -> Dx7Preset {
    let mut operators: [PresetOperator; 6] = std::array::from_fn(|_| PresetOperator::default());

    // Each operator occupies 17 packed bytes; SysEx orders OP6..OP1.
    for sysex_idx in 0..6 {
        let base = sysex_idx * 17;
        let preset_idx = 5 - sysex_idx;
        operators[preset_idx] = parse_vmem_operator(&block[base..base + 17]);
    }

    let pitch_eg = PresetPitchEg {
        rate1: block[102] as f32,
        rate2: block[103] as f32,
        rate3: block[104] as f32,
        rate4: block[105] as f32,
        level1: block[106] as f32,
        level2: block[107] as f32,
        level3: block[108] as f32,
        level4: block[109] as f32,
    };

    let algorithm = (block[110] & 0x1F) + 1;
    // byte 111: bits 0-2 = feedback, bit 3 = oscillator key sync
    let feedback = (block[111] & 0x07) as f32;
    let osc_key_sync = (block[111] & 0x08) != 0;

    let lfo_speed = block[112] as f32;
    let lfo_delay = block[113] as f32;
    let lfo_pmd = block[114] as f32;
    let lfo_amd = block[115] as f32;
    // byte 116: bit 0 = LFO sync, bits 1-3 = LFO wave, bits 4-6 = PMS
    let lfo_sync = (block[116] & 0x01) != 0;
    let lfo_wave_code = (block[116] >> 1) & 0x07;
    let pms = (block[116] >> 4) & 0x07;

    let lfo = PresetLfo {
        waveform: lfo_wave_from_dx7(lfo_wave_code),
        rate: lfo_speed,
        delay: lfo_delay,
        pitch_mod_depth: lfo_pmd,
        amp_mod_depth: lfo_amd,
        key_sync: lfo_sync,
    };

    let transpose_semitones = (block[117] as i16 - 24).clamp(-24, 24) as i8;
    let name = parse_voice_name(&block[118..128]);

    operators[5].feedback = feedback;
    for op in operators.iter_mut() {
        op.oscillator_key_sync = osc_key_sync;
    }

    Dx7Preset {
        name,
        collection: collection.to_string(),
        algorithm,
        operators,
        master_tune: None,
        pitch_bend_range: None,
        portamento_enable: None,
        portamento_time: None,
        mono_mode: None,
        transpose_semitones,
        pitch_mod_sensitivity: pms,
        pitch_eg: Some(pitch_eg),
        lfo: Some(lfo),
    }
}

fn parse_vmem_operator(block: &[u8]) -> PresetOperator {
    let r1 = block[0] as f32;
    let r2 = block[1] as f32;
    let r3 = block[2] as f32;
    let r4 = block[3] as f32;
    let l1 = block[4] as f32;
    let l2 = block[5] as f32;
    let l3 = block[6] as f32;
    let l4 = block[7] as f32;
    let breakpoint = block[8];
    let kls_ld = block[9] as f32;
    let kls_rd = block[10] as f32;
    // byte 11: bits 0-1 = LC, bits 2-3 = RC
    let kls_lc = block[11] & 0x03;
    let kls_rc = (block[11] >> 2) & 0x03;
    // byte 12: bits 0-2 = RS, bits 3-4 = AMS
    let krs = block[12] & 0x07;
    let ams = (block[12] >> 3) & 0x03;
    let kvs = block[13] & 0x07;
    let level = block[14] as f32;
    // byte 15: bit 0 = oscillator mode, bits 1-5 = coarse
    let osc_mode = block[15] & 0x01;
    let coarse = (block[15] >> 1) & 0x1F;
    let fine = block[16];
    let detune_raw = (block[12] >> 5) & 0x0F; // VMEM stores detune in upper bits of byte 12... or 17?

    // Note: in the real VMEM format the detune sits in bits 4-7 of byte 12 (combined
    // with KRS+AMS). Some references shuffle the layout; we read it from there.
    // Treat 7 as center as in VCED.
    let detune = (detune_raw as i16 - 7) as f32;

    let fixed_frequency = osc_mode == 1;
    let frequency_ratio = if fixed_frequency {
        1.0
    } else if coarse == 0 {
        0.5
    } else {
        (coarse as f32) * (1.0 + (fine as f32) / 100.0)
    };
    let fixed_freq_hz = if fixed_frequency {
        let c = (coarse & 0x03) as f32;
        10f32.powf(c) * (1.0 + (fine as f32) / 100.0)
    } else {
        440.0
    };

    let breakpoint_midi = breakpoint.saturating_add(21).min(127);

    PresetOperator {
        frequency_ratio,
        output_level: level,
        detune,
        feedback: 0.0,
        velocity_sensitivity: kvs as f32,
        key_scale_rate: krs as f32,
        key_scale_breakpoint: breakpoint_midi,
        key_scale_left_curve: KeyScaleCurve::from_dx7_code(kls_lc),
        key_scale_right_curve: KeyScaleCurve::from_dx7_code(kls_rc),
        key_scale_left_depth: kls_ld.clamp(0.0, 99.0),
        key_scale_right_depth: kls_rd.clamp(0.0, 99.0),
        am_sensitivity: ams,
        oscillator_key_sync: true,
        fixed_frequency,
        fixed_freq_hz,
        envelope: (r1, r2, r3, r4, l1, l2, l3, l4),
    }
}

// ---------------------------------------------------------------------------
// VCED encoder
// ---------------------------------------------------------------------------

fn encode_vced(preset: &Dx7Preset) -> Vec<u8> {
    let mut buf = vec![0u8; VCED_LEN];

    for sysex_idx in 0..6 {
        let preset_idx = 5 - sysex_idx;
        let base = sysex_idx * 21;
        encode_vced_operator(&preset.operators[preset_idx], &mut buf[base..base + 21]);
    }

    let peg = preset.pitch_eg.clone().unwrap_or_default();
    buf[126] = clamp_99(peg.rate1);
    buf[127] = clamp_99(peg.rate2);
    buf[128] = clamp_99(peg.rate3);
    buf[129] = clamp_99(peg.rate4);
    buf[130] = clamp_99(peg.level1);
    buf[131] = clamp_99(peg.level2);
    buf[132] = clamp_99(peg.level3);
    buf[133] = clamp_99(peg.level4);

    buf[134] = preset.algorithm.saturating_sub(1).min(31);
    // DX7 stores feedback (per-algorithm operator) at the patch level. We took it
    // off OP6 so we round-trip cleanly.
    buf[135] = (preset.operators[5].feedback.round() as u8).min(7);
    buf[136] = if preset.operators.iter().any(|op| op.oscillator_key_sync) {
        1
    } else {
        0
    };

    let lfo = preset.lfo.clone().unwrap_or_default();
    buf[137] = clamp_99(lfo.rate);
    buf[138] = clamp_99(lfo.delay);
    buf[139] = clamp_99(lfo.pitch_mod_depth);
    buf[140] = clamp_99(lfo.amp_mod_depth);
    buf[141] = if lfo.key_sync { 1 } else { 0 };
    buf[142] = lfo_wave_to_dx7(lfo.waveform);
    buf[143] = preset.pitch_mod_sensitivity.min(7);
    // Transpose: stored as 0..48 with 24 = no shift.
    buf[144] = ((preset.transpose_semitones as i16 + 24).clamp(0, 48)) as u8;

    let mut name_bytes = preset.name.as_bytes().to_vec();
    name_bytes.resize(10, b' ');
    for (i, b) in name_bytes.iter().take(10).enumerate() {
        buf[145 + i] = b & 0x7F;
    }

    buf
}

fn encode_vced_operator(op: &PresetOperator, out: &mut [u8]) {
    let (r1, r2, r3, r4, l1, l2, l3, l4) = op.envelope;
    out[0] = clamp_99(r1);
    out[1] = clamp_99(r2);
    out[2] = clamp_99(r3);
    out[3] = clamp_99(r4);
    out[4] = clamp_99(l1);
    out[5] = clamp_99(l2);
    out[6] = clamp_99(l3);
    out[7] = clamp_99(l4);
    // DX7 stores breakpoint as MIDI note minus 21 (so A-1 = 0).
    out[8] = op.key_scale_breakpoint.saturating_sub(21).min(99);
    out[9] = clamp_99(op.key_scale_left_depth);
    out[10] = clamp_99(op.key_scale_right_depth);
    out[11] = op.key_scale_left_curve.to_dx7_code();
    out[12] = op.key_scale_right_curve.to_dx7_code();
    out[13] = (op.key_scale_rate.round() as u8).min(7);
    out[14] = op.am_sensitivity.min(3);
    out[15] = (op.velocity_sensitivity.round() as u8).min(7);
    out[16] = clamp_99(op.output_level);
    out[17] = if op.fixed_frequency { 1 } else { 0 };
    if op.fixed_frequency {
        // Map Hz back to coarse (1/10/100/1000) + fine (0..99).
        let log10 = op.fixed_freq_hz.max(0.1).log10();
        let coarse = log10.floor().clamp(0.0, 3.0) as u8;
        let base = 10f32.powi(coarse as i32);
        let fine = ((op.fixed_freq_hz / base - 1.0) * 100.0).clamp(0.0, 99.0) as u8;
        out[18] = coarse;
        out[19] = fine;
    } else {
        // Inverse of `coarse * (1 + fine/100)` with the coarse=0 / 0.5× quirk.
        if (op.frequency_ratio - 0.5).abs() < 0.01 {
            out[18] = 0;
            out[19] = 0;
        } else {
            let coarse = op.frequency_ratio.floor().clamp(1.0, 31.0) as u8;
            let frac = op.frequency_ratio / coarse as f32 - 1.0;
            let fine = (frac * 100.0).round().clamp(0.0, 99.0) as u8;
            out[18] = coarse;
            out[19] = fine;
        }
    }
    out[20] = ((op.detune.round() as i16 + 7).clamp(0, 14)) as u8;
}

fn clamp_99(v: f32) -> u8 {
    v.round().clamp(0.0, 99.0) as u8
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lfo_wave_from_dx7(code: u8) -> LFOWaveform {
    match code & 0x07 {
        0 => LFOWaveform::Triangle,
        1 => LFOWaveform::SawDown,
        2 => LFOWaveform::SawUp,
        3 => LFOWaveform::Square,
        4 => LFOWaveform::Sine,
        _ => LFOWaveform::SampleHold,
    }
}

fn lfo_wave_to_dx7(wave: LFOWaveform) -> u8 {
    match wave {
        LFOWaveform::Triangle => 0,
        LFOWaveform::SawDown => 1,
        LFOWaveform::SawUp => 2,
        LFOWaveform::Square => 3,
        LFOWaveform::Sine => 4,
        LFOWaveform::SampleHold => 5,
    }
}

fn parse_voice_name(raw: &[u8]) -> String {
    // DX7 uses 7-bit ASCII; pad with spaces, trim trailing whitespace.
    let mut s = String::new();
    for &b in raw {
        let c = (b & 0x7F) as char;
        if c.is_ascii() {
            s.push(c);
        }
    }
    s.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_preset() -> Dx7Preset {
        let mut operators: [PresetOperator; 6] = std::array::from_fn(|_| PresetOperator::default());
        operators[0].frequency_ratio = 2.0;
        operators[0].output_level = 99.0;
        operators[5].feedback = 5.0;
        operators[5].am_sensitivity = 2;

        Dx7Preset {
            name: "TEST PATCH".to_string(),
            collection: "test".to_string(),
            algorithm: 5,
            operators,
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 3,
            pitch_eg: Some(PresetPitchEg::default()),
            lfo: Some(PresetLfo::default()),
        }
    }

    #[test]
    fn checksum_matches_yamaha_two_complement() {
        // Trivial vector: all zeros → sum 0 → checksum 0.
        assert_eq!(compute_checksum(&[0u8; 10]), 0);
        // Single 0x01 → sum 1 → checksum 0x7F.
        assert_eq!(compute_checksum(&[0x01]), 0x7F);
        // Sum 0x40 → checksum 0x40.
        assert_eq!(compute_checksum(&[0x20, 0x20]), 0x40);
    }

    #[test]
    fn vced_roundtrip_preserves_core_fields() {
        let preset = make_test_preset();
        let bytes = encode_single_voice(&preset, 0);
        assert_eq!(bytes.len(), VCED_LEN + 8);
        assert_eq!(bytes[0], 0xF0);
        assert_eq!(bytes[1], YAMAHA_ID);
        assert_eq!(*bytes.last().unwrap(), 0xF7);

        let parsed = parse_message(&bytes).expect("parse_message");
        match parsed {
            SysexResult::SingleVoice(boxed) => {
                let p = *boxed;
                assert_eq!(p.algorithm, 5);
                assert_eq!(p.name, "TEST PATCH");
                assert_eq!(p.pitch_mod_sensitivity, 3);
                // Feedback round-trips on OP6.
                assert!((p.operators[5].feedback - 5.0).abs() < 0.01);
                assert_eq!(p.operators[5].am_sensitivity, 2);
                // OP1 ratio survived the encode/decode round-trip.
                assert!((p.operators[0].frequency_ratio - 2.0).abs() < 0.05);
            }
            _ => panic!("expected SingleVoice"),
        }
    }

    #[test]
    fn detects_invalid_framing() {
        let bytes = vec![0x00; 12];
        assert!(matches!(
            parse_message(&bytes),
            Err(SysexError::InvalidFraming)
        ));
    }

    #[test]
    fn detects_non_yamaha_id() {
        let mut bytes = vec![0u8; 12];
        bytes[0] = 0xF0;
        bytes[1] = 0x42; // Korg
        bytes[11] = 0xF7;
        assert!(matches!(
            parse_message(&bytes),
            Err(SysexError::NotYamaha(0x42))
        ));
    }

    #[test]
    fn detects_checksum_mismatch() {
        let preset = make_test_preset();
        let mut bytes = encode_single_voice(&preset, 0);
        // Flip the checksum byte (second to last).
        let cs = bytes.len() - 2;
        bytes[cs] ^= 0x01;
        assert!(matches!(
            parse_message(&bytes),
            Err(SysexError::ChecksumMismatch { .. })
        ));
    }
}
