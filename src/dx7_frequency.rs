/// Get the closest DX7 frequency ratio to a given value
pub fn quantize_frequency_ratio(ratio: f32) -> f32 {
    // Special cases for fixed ratios
    if ratio < 0.75 {
        return 0.50;
    }
    if ratio < 1.5 {
        return 1.00;
    }

    // For higher values, round to nearest integer
    ratio.round().clamp(1.0, 31.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn very_low_ratios_snap_to_half() {
        assert_eq!(quantize_frequency_ratio(0.0), 0.5);
        assert_eq!(quantize_frequency_ratio(0.5), 0.5);
        assert_eq!(quantize_frequency_ratio(0.74), 0.5);
    }

    #[test]
    fn mid_low_ratios_snap_to_unity() {
        assert_eq!(quantize_frequency_ratio(0.75), 1.0);
        assert_eq!(quantize_frequency_ratio(1.0), 1.0);
        assert_eq!(quantize_frequency_ratio(1.49), 1.0);
    }

    #[test]
    fn higher_ratios_round_to_integer() {
        assert_eq!(quantize_frequency_ratio(1.5), 2.0);
        assert_eq!(quantize_frequency_ratio(2.4), 2.0);
        assert_eq!(quantize_frequency_ratio(2.6), 3.0);
        assert_eq!(quantize_frequency_ratio(7.0), 7.0);
    }

    #[test]
    fn ratios_above_31_clamp() {
        assert_eq!(quantize_frequency_ratio(50.0), 31.0);
        assert_eq!(quantize_frequency_ratio(1000.0), 31.0);
    }

    #[test]
    fn integer_round_trips() {
        for r in 2..=31 {
            let f = r as f32;
            assert_eq!(quantize_frequency_ratio(f), f);
        }
    }
}
