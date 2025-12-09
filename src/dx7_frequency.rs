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
