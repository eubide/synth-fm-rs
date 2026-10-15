/// DX7 fixed-mode frequency in Hz: `10^(coarse + fine/100)`, coarse 0..3
/// selecting 1/10/100/1000 Hz and fine 0..99 multiplying by up to ~9.77
/// (Dexed `Dx7Note::osc_freq`, mode 1).
pub fn fixed_frequency_hz(coarse: u8, fine: u8) -> f32 {
    10f32.powf((coarse & 3) as f32 + fine.min(99) as f32 / 100.0)
}

/// Nearest DX7 fixed-mode `(coarse, fine)` for a frequency in Hz.
pub fn fixed_coarse_fine(hz: f32) -> (u8, u8) {
    let steps = (hz.max(1.0).log10() * 100.0).round().clamp(0.0, 399.0) as u16;
    ((steps / 100) as u8, (steps % 100) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_frequency_is_exponential_in_fine() {
        assert!((fixed_frequency_hz(0, 0) - 1.0).abs() < 1e-4);
        assert!((fixed_frequency_hz(0, 99) - 9.772).abs() < 1e-3);
        assert!((fixed_frequency_hz(3, 66) - 4570.88).abs() < 0.05);
    }

    #[test]
    fn fixed_coarse_fine_inverts_fixed_frequency_hz() {
        for (coarse, fine) in [(0, 0), (1, 37), (2, 50), (3, 66), (3, 99)] {
            assert_eq!(
                fixed_coarse_fine(fixed_frequency_hz(coarse, fine)),
                (coarse, fine)
            );
        }
        assert_eq!(fixed_coarse_fine(0.5), (0, 0));
        assert_eq!(fixed_coarse_fine(20_000.0), (3, 99));
    }
}
