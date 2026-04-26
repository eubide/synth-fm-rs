use crate::operator::Operator;

/// Direct hardcoded implementation of all 32 DX7 algorithms
/// Each algorithm is implemented as a specific function for maximum clarity and performance
pub fn process_algorithm(algorithm_number: u8, ops: &mut [Operator; 6]) -> f32 {
    match algorithm_number {
        1 => algorithm_1(ops),
        2 => algorithm_2(ops),
        3 => algorithm_3(ops),
        4 => algorithm_4(ops),
        5 => algorithm_5(ops),
        6 => algorithm_6(ops),
        7 => algorithm_7(ops),
        8 => algorithm_8(ops),
        9 => algorithm_9(ops),
        10 => algorithm_10(ops),
        11 => algorithm_11(ops),
        12 => algorithm_12(ops),
        13 => algorithm_13(ops),
        14 => algorithm_14(ops),
        15 => algorithm_15(ops),
        16 => algorithm_16(ops),
        17 => algorithm_17(ops),
        18 => algorithm_18(ops),
        19 => algorithm_19(ops),
        20 => algorithm_20(ops),
        21 => algorithm_21(ops),
        22 => algorithm_22(ops),
        23 => algorithm_23(ops),
        24 => algorithm_24(ops),
        25 => algorithm_25(ops),
        26 => algorithm_26(ops),
        27 => algorithm_27(ops),
        28 => algorithm_28(ops),
        29 => algorithm_29(ops),
        30 => algorithm_30(ops),
        31 => algorithm_31(ops),
        32 => algorithm_32(ops),
        _ => algorithm_1(ops), // Default fallback
    }
}

/// Algorithm 1: Two Stacks
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,5), (6,6)]
fn algorithm_1(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Stack 2: Op6 -> Op5 -> Op4 -> Op3 (with Op6 feedback)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 2: Stack + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,5), (2,2)]
fn algorithm_2(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op2 -> Op1 (with Op2 feedback)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Stack 2: Op6 -> Op5 -> Op4 -> Op3
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 3: Dual Stacks
/// Carriers: [1, 4] - Connections: [(2,1), (3,2), (5,4), (6,5), (6,6)]
fn algorithm_3(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op3 -> Op2 -> Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Stack 2: Op6 -> Op5 -> Op4 (with Op6 feedback)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 4: Stack Loop (cross-feedback)
/// Carriers: [1, 4] - Connections: [(3,2), (2,1), (6,5), (5,4)] - Feedback: Op4→Op6 loop
fn algorithm_4(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op3 -> Op2 -> Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Stack 2: Op6 -> Op5 -> Op4 with cross-feedback loop (Op4 output → Op6 input)
    // Op4's averaged previous output feeds Op6, depth controlled by Op4's feedback param
    let fb_depth = ops[3].feedback;
    let op4_cross_fb = ops[3].cross_feedback_signal(fb_depth);
    let op6_out = ops[5].process(op4_cross_fb);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process_no_self_feedback(op5_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 5: Three Pairs
/// Carriers: [1, 3, 5] - Connections: [(2,1), (4,3), (6,5)] - Feedback: Op6
fn algorithm_5(ops: &mut [Operator; 6]) -> f32 {
    // Three independent modulator-carrier pairs
    // Op2 -> Op1 (carrier)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op4 -> Op3 (carrier)
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Op6 (feedback) -> Op5 (carrier)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    (op1_out + op3_out + op5_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 6: Three Pairs (cross-feedback)
/// Carriers: [1, 3, 5] - Connections: [(2,1), (4,3), (6,5)] - Feedback: Op5→Op6 loop
fn algorithm_6(ops: &mut [Operator; 6]) -> f32 {
    // Three modulator-carrier pairs, with cross-feedback (Op5 output → Op6 input)
    // Op2 -> Op1 (carrier)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op4 -> Op3 (carrier)
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Op6 -> Op5 (carrier) with cross-feedback (Op5's previous output → Op6 input)
    // Depth controlled by Op6's feedback param (presets set feedback_op: 6)
    let fb_depth = ops[5].feedback;
    let op5_cross_fb = ops[4].cross_feedback_signal(fb_depth);
    let op6_out = ops[5].process_no_self_feedback(op5_cross_fb);
    let op5_out = ops[4].process(op6_out);

    (op1_out + op3_out + op5_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 7: Dual + Stack
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,5), (6,6)]
fn algorithm_7(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Stack: Op6 -> Op5 -> Op4 -> Op3 (with Op6 feedback)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out + op5_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 8: Dual Split
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,5), (4,4)]
fn algorithm_8(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op6 -> Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op4 with feedback and Op5 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out + op5_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 9: Dual + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,5), (2,2)]
fn algorithm_9(ops: &mut [Operator; 6]) -> f32 {
    // Op2 with feedback -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op6 -> Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op4 and Op5 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out + op5_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 10: Split Stack
/// Carriers: [1, 4] - Connections: [(5,4), (6,4), (3,2), (2,1), (3,3)]
fn algorithm_10(ops: &mut [Operator; 6]) -> f32 {
    // Op3 with feedback
    let op3_out = ops[2].process(0.0);

    // Op3 -> Op2 -> Op1 (first carrier path)
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Op5 and Op6 -> Op4 (second carrier path)
    let op5_out = ops[4].process(0.0);
    let op6_out = ops[5].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 11: Stack + Dual
/// Carriers: [1, 4] - Connections: [(2,1), (3,2), (5,4), (6,4), (6,6)]
fn algorithm_11(ops: &mut [Operator; 6]) -> f32 {
    // Op3 -> Op2 -> Op1 (first carrier path)
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Op6 with feedback and Op5 -> Op4 (second carrier path)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 12: Triple Mod
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,3), (2,2)]
fn algorithm_12(ops: &mut [Operator; 6]) -> f32 {
    // Op2 with feedback -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op4, Op5, Op6 -> Op3
    let op4_out = ops[3].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op6_out = ops[5].process(0.0);
    let op3_out = ops[2].process(op4_out + op5_out + op6_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 13: Triple Fan
/// Carriers: [3, 1] - Connections: [(2,1), (4,3), (5,3), (6,3), (6,6)]
fn algorithm_13(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op4, Op5, Op6 with feedback -> Op3
    let op6_out = ops[5].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op3_out = ops[2].process(op4_out + op5_out + op6_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 14: Dual Stack
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,4), (6,6)]
fn algorithm_14(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op6 with feedback and Op5 -> Op4 -> Op3
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 15: Stack + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,4), (2,2)]
fn algorithm_15(ops: &mut [Operator; 6]) -> f32 {
    // Op2 with feedback -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op6 and Op5 -> Op4 -> Op3
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 16: Tree + Self
/// Carriers: [1] - Connections: [(2,1), (3,1), (5,1), (4,3), (6,5), (6,6)]
fn algorithm_16(ops: &mut [Operator; 6]) -> f32 {
    // Op6 with feedback -> Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op4 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Op2, Op3, Op5 -> Op1
    let op2_out = ops[1].process(0.0);
    ops[0].process(op2_out + op3_out + op5_out)
}

/// Algorithm 17: Tree Mod
/// Carriers: [1] - Connections: [(2,1), (3,1), (5,1), (4,3), (6,5), (2,2)]
fn algorithm_17(ops: &mut [Operator; 6]) -> f32 {
    // Op6 -> Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op4 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Op2 with feedback, Op3, Op5 -> Op1
    let op2_out = ops[1].process(0.0);
    ops[0].process(op2_out + op3_out + op5_out)
}

/// Algorithm 18: Quad + Stack
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,1), (5,4), (6,5), (3,3)]
fn algorithm_18(ops: &mut [Operator; 6]) -> f32 {
    // Op6 -> Op5 -> Op4
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    // Op3 with feedback
    let op3_out = ops[2].process(0.0);

    // Op2, Op3, Op4 -> Op1
    let op2_out = ops[1].process(0.0);
    ops[0].process(op2_out + op3_out + op4_out)
}

/// Algorithm 19: Fan + Stack
/// Carriers: [1, 4, 5] - Connections: [(3,2), (2,1), (6,5), (6,4)] - Feedback: Op6
fn algorithm_19(ops: &mut [Operator; 6]) -> f32 {
    // Op6 (feedback) modulates both Op5 and Op4
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out); // Op6 -> Op5 (carrier)
    let op4_out = ops[3].process(op6_out); // Op6 -> Op4 (carrier)

    // Op3 -> Op2 -> Op1 (carrier)
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    (op1_out + op4_out + op5_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 20: Triple + Dual
/// Carriers: [1, 2, 4] - Connections: [(3,1), (3,2), (5,4), (6,4), (3,3)]
fn algorithm_20(ops: &mut [Operator; 6]) -> f32 {
    // Op3 with feedback
    let op3_out = ops[2].process(0.0);

    // Op5 and Op6 -> Op4 (carrier)
    let op5_out = ops[4].process(0.0);
    let op6_out = ops[5].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    // Op3 -> Op1 and Op2 (carriers)
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op3_out);

    (op1_out + op2_out + op4_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 21: Quad + Dual
/// Carriers: [1, 2, 4, 5] - Connections: [(3,1), (3,2), (6,4), (6,5), (3,3)]
fn algorithm_21(ops: &mut [Operator; 6]) -> f32 {
    // Op3 with feedback
    let op3_out = ops[2].process(0.0);

    // Op6 -> Op4 and Op5 (carriers)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op6_out);

    // Op3 -> Op1 and Op2 (carriers)
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op3_out);

    (op1_out + op2_out + op4_out + op5_out) * 0.5 // √4 = 2.0, inverse = 0.5
}

/// Algorithm 22: Quad + Stack
/// Carriers: [1, 3, 4, 5] - Connections: [(2,1), (6,3), (6,4), (6,5), (6,6)]
fn algorithm_22(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1 (carrier)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op6 with feedback -> Op3, Op4, Op5 (carriers)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op6_out);
    let op3_out = ops[2].process(op6_out);

    (op1_out + op3_out + op4_out + op5_out) * 0.5 // √4 = 2.0, inverse = 0.5
}

/// Algorithm 23: Quad + Self
/// Carriers: [1, 2, 4, 5] - Connections: [(3,2), (6,4), (6,5), (6,6)]
fn algorithm_23(ops: &mut [Operator; 6]) -> f32 {
    // Op3 -> Op2 (carrier)
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);

    // Op6 with feedback -> Op4 and Op5 (carriers)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op6_out);

    // Op1 is carrier (no modulation)
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op4_out + op5_out) * 0.5 // √4 = 2.0, inverse = 0.5
}

/// Algorithm 24: Penta + Self
/// Carriers: [1, 2, 3, 4, 5] - Connections: [(6,3), (6,4), (6,5), (6,6)]
fn algorithm_24(ops: &mut [Operator; 6]) -> f32 {
    // Op6 with feedback -> Op3, Op4, Op5 (carriers)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op6_out);
    let op3_out = ops[2].process(op6_out);

    // Op1 and Op2 are carriers (no modulation)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out) * 0.45 // √5 = 2.24, inverse = 0.45
}

/// Algorithm 25: Penta + Dual
/// Carriers: [1, 2, 3, 4, 5] - Connections: [(6,4), (6,5), (6,6)]
fn algorithm_25(ops: &mut [Operator; 6]) -> f32 {
    // Op6 with feedback -> Op4 and Op5 (carriers)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op6_out);

    // Op1, Op2, Op3 are carriers (no modulation)
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out) * 0.45 // √5 = 2.24, inverse = 0.45
}

/// Algorithm 26: Triple + Self
/// Carriers: [1, 2, 4] - Connections: [(3,2), (5,4), (6,4), (6,6)]
fn algorithm_26(ops: &mut [Operator; 6]) -> f32 {
    // Op3 -> Op2 (carrier)
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);

    // Op6 with feedback and Op5 -> Op4 (carrier)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    // Op1 is carrier (no modulation)
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op4_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 27: Triple Split
/// Carriers: [1, 2, 4] - Connections: [(3,2), (5,4), (6,4), (3,3)]
fn algorithm_27(ops: &mut [Operator; 6]) -> f32 {
    // Op3 with feedback -> Op2 (carrier)
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);

    // Op5 and Op6 -> Op4 (carrier)
    let op5_out = ops[4].process(0.0);
    let op6_out = ops[5].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    // Op1 is carrier (no modulation)
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op4_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 28: Triple + Stack
/// Carriers: [1, 3, 6] - Connections: [(2,1), (4,3), (5,4), (5,5)]
fn algorithm_28(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1 (carrier)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Op5 with feedback -> Op4 -> Op3 (carrier)
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    // Op6 is carrier (no modulation)
    let op6_out = ops[5].process(0.0);

    (op1_out + op3_out + op6_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 29: Quad + Stack
/// Carriers: [1, 2, 3, 5] - Connections: [(4,3), (6,5), (6,6)]
fn algorithm_29(ops: &mut [Operator; 6]) -> f32 {
    // Op4 -> Op3 (carrier)
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Op6 with feedback -> Op5 (carrier)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op1 and Op2 are carriers (no modulation)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op5_out) * 0.5 // √4 = 2.0, inverse = 0.5
}

/// Algorithm 30: Quad + Self
/// Carriers: [1, 2, 3, 6] - Connections: [(4,3), (5,4), (5,5)]
fn algorithm_30(ops: &mut [Operator; 6]) -> f32 {
    // Op5 with feedback -> Op4 -> Op3 (carrier)
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    // Op1, Op2, Op6 are carriers (no modulation)
    let op6_out = ops[5].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op6_out) * 0.5 // √4 = 2.0, inverse = 0.5
}

/// Algorithm 31: Five Carriers + Modulator
/// Carriers: [1, 2, 3, 4, 5] - Connections: [(6,5)] - Feedback: Op6
fn algorithm_31(ops: &mut [Operator; 6]) -> f32 {
    // Op6 (feedback) modulates Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op1-4 are standalone carriers
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out) * 0.45 // √5 = 2.24, inverse = 0.45
}

/// Algorithm 32: All Carriers
/// Carriers: [1, 2, 3, 4, 5, 6] - Connections: [(6,6)]
fn algorithm_32(ops: &mut [Operator; 6]) -> f32 {
    // All operators are carriers (with Op6 feedback)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out + op6_out) * 0.41 // √6 = 2.45, inverse = 0.41
}

/// Get algorithm name for display
pub fn get_algorithm_name(algorithm_number: u8) -> &'static str {
    match algorithm_number {
        1 => "1: Two Stacks",
        2 => "2: Stack + Self",
        3 => "3: Dual Stacks",
        4 => "4: Stack Loop",
        5 => "5: Three Pairs",
        6 => "6: Three Pairs FB",
        7 => "7: Dual + Stack",
        8 => "8: Dual Split",
        9 => "9: Dual + Self",
        10 => "10: Split Stack",
        11 => "11: Stack + Dual",
        12 => "12: Triple Mod",
        13 => "13: Triple Fan",
        14 => "14: Dual Stack",
        15 => "15: Stack + Self",
        16 => "16: Tree + Self",
        17 => "17: Tree Mod",
        18 => "18: Quad + Stack",
        19 => "19: Fan + Stack",
        20 => "20: Triple + Dual",
        21 => "21: Quad + Dual",
        22 => "22: Quad + Stack",
        23 => "23: Quad + Self",
        24 => "24: Penta + Self",
        25 => "25: Penta + Dual",
        26 => "26: Triple + Self",
        27 => "27: Triple Split",
        28 => "28: Triple + Stack",
        29 => "29: Quad + Stack",
        30 => "30: Quad + Self",
        31 => "31: Five + Mod",
        32 => "32: All Carriers",
        _ => "1: Two Stacks",
    }
}

/// Algorithm structure information for visualization
#[derive(Debug, Clone)]
pub struct AlgorithmInfo {
    /// Which operators are carriers (1-indexed)
    pub carriers: Vec<u8>,
    /// Connections: (from, to) where 'from' modulates 'to' (1-indexed)
    pub connections: Vec<(u8, u8)>,
    /// Which operator has self-feedback (1-indexed), 0 if none
    pub feedback_op: u8,
}

/// Get algorithm structure for visualization
pub fn get_algorithm_info(algorithm_number: u8) -> AlgorithmInfo {
    match algorithm_number {
        1 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 4), (6, 5)],
            feedback_op: 6,
        },
        2 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 4), (6, 5)],
            feedback_op: 2,
        },
        3 => AlgorithmInfo {
            carriers: vec![1, 4],
            connections: vec![(2, 1), (3, 2), (5, 4), (6, 5)],
            feedback_op: 6,
        },
        4 => AlgorithmInfo {
            carriers: vec![1, 4],
            connections: vec![(3, 2), (2, 1), (6, 5), (5, 4)],
            feedback_op: 4, // Cross-feedback: Op4→Op6 loop
        },
        5 => AlgorithmInfo {
            carriers: vec![1, 3, 5],
            connections: vec![(2, 1), (4, 3), (6, 5)],
            feedback_op: 6,
        },
        6 => AlgorithmInfo {
            carriers: vec![1, 3, 5],
            connections: vec![(2, 1), (4, 3), (6, 5)],
            feedback_op: 6, // Cross-feedback: Op5→Op6 loop
        },
        7 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 3), (6, 5)],
            feedback_op: 6,
        },
        8 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 3), (6, 5)],
            feedback_op: 4,
        },
        9 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 3), (6, 5)],
            feedback_op: 2,
        },
        10 => AlgorithmInfo {
            carriers: vec![1, 4],
            connections: vec![(2, 1), (3, 2), (5, 4), (6, 4)],
            feedback_op: 3,
        },
        11 => AlgorithmInfo {
            carriers: vec![1, 4],
            connections: vec![(2, 1), (3, 2), (5, 4), (6, 4)],
            feedback_op: 6,
        },
        12 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 3), (6, 3)],
            feedback_op: 2,
        },
        13 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 3), (6, 3)],
            feedback_op: 6,
        },
        14 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 4), (6, 4)],
            feedback_op: 6,
        },
        15 => AlgorithmInfo {
            carriers: vec![1, 3],
            connections: vec![(2, 1), (4, 3), (5, 4), (6, 4)],
            feedback_op: 2,
        },
        16 => AlgorithmInfo {
            carriers: vec![1],
            connections: vec![(2, 1), (3, 1), (4, 3), (5, 1), (6, 5)],
            feedback_op: 6,
        },
        17 => AlgorithmInfo {
            carriers: vec![1],
            connections: vec![(2, 1), (3, 1), (4, 3), (5, 1), (6, 5)],
            feedback_op: 2,
        },
        18 => AlgorithmInfo {
            carriers: vec![1],
            connections: vec![(2, 1), (3, 1), (4, 1), (5, 4), (6, 5)],
            feedback_op: 3,
        },
        19 => AlgorithmInfo {
            carriers: vec![1, 4, 5],
            connections: vec![(3, 2), (2, 1), (6, 5), (6, 4)],
            feedback_op: 6,
        },
        20 => AlgorithmInfo {
            carriers: vec![1, 2, 4],
            connections: vec![(3, 1), (3, 2), (5, 4), (6, 4)],
            feedback_op: 3,
        },
        21 => AlgorithmInfo {
            carriers: vec![1, 2, 4, 5],
            connections: vec![(3, 1), (3, 2), (6, 4), (6, 5)],
            feedback_op: 3,
        },
        22 => AlgorithmInfo {
            carriers: vec![1, 3, 4, 5],
            connections: vec![(2, 1), (6, 3), (6, 4), (6, 5)],
            feedback_op: 6,
        },
        23 => AlgorithmInfo {
            carriers: vec![1, 2, 4, 5],
            connections: vec![(3, 2), (6, 4), (6, 5)],
            feedback_op: 6,
        },
        24 => AlgorithmInfo {
            carriers: vec![1, 2, 3, 4, 5],
            connections: vec![(6, 3), (6, 4), (6, 5)],
            feedback_op: 6,
        },
        25 => AlgorithmInfo {
            carriers: vec![1, 2, 3, 4, 5],
            connections: vec![(6, 4), (6, 5)],
            feedback_op: 6,
        },
        26 => AlgorithmInfo {
            carriers: vec![1, 2, 4],
            connections: vec![(3, 2), (5, 4), (6, 4)],
            feedback_op: 6,
        },
        27 => AlgorithmInfo {
            carriers: vec![1, 2, 4],
            connections: vec![(3, 2), (5, 4), (6, 4)],
            feedback_op: 3,
        },
        28 => AlgorithmInfo {
            carriers: vec![1, 3, 6],
            connections: vec![(2, 1), (4, 3), (5, 4)],
            feedback_op: 5,
        },
        29 => AlgorithmInfo {
            carriers: vec![1, 2, 3, 5],
            connections: vec![(4, 3), (6, 5)],
            feedback_op: 6,
        },
        30 => AlgorithmInfo {
            carriers: vec![1, 2, 3, 6],
            connections: vec![(4, 3), (5, 4)],
            feedback_op: 5,
        },
        31 => AlgorithmInfo {
            carriers: vec![1, 2, 3, 4, 5],
            connections: vec![(6, 5)],
            feedback_op: 6,
        },
        32 => AlgorithmInfo {
            carriers: vec![1, 2, 3, 4, 5, 6],
            connections: vec![],
            feedback_op: 6,
        },
        _ => get_algorithm_info(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    fn build_ops() -> [Operator; 6] {
        [
            Operator::new(SR),
            Operator::new(SR),
            Operator::new(SR),
            Operator::new(SR),
            Operator::new(SR),
            Operator::new(SR),
        ]
    }

    fn triggered_ops() -> [Operator; 6] {
        let mut ops = build_ops();
        for op in ops.iter_mut() {
            op.envelope.rate1 = 99.0;
            op.trigger(440.0, 1.0, 60);
        }
        ops
    }

    fn run_algorithm_for_samples(alg: u8, samples: usize) -> (f32, f32) {
        let mut ops = triggered_ops();
        // Warm up envelope to steady state
        for _ in 0..2048 {
            process_algorithm(alg, &mut ops);
        }
        let mut peak = 0.0_f32;
        let mut energy = 0.0_f32;
        for _ in 0..samples {
            let s = process_algorithm(alg, &mut ops);
            peak = peak.max(s.abs());
            energy += s * s;
        }
        (peak, energy)
    }

    #[test]
    fn every_algorithm_produces_audio() {
        for alg in 1..=32u8 {
            let (peak, energy) = run_algorithm_for_samples(alg, 2048);
            assert!(
                peak > 1e-6,
                "algorithm {alg} produced no audio (peak={peak})"
            );
            assert!(energy > 1e-6, "algorithm {alg} has no signal energy");
            // Output should stay within sane bounds (post-mix scaling).
            assert!(peak < 5.0, "algorithm {alg} peak too high: {peak}");
        }
    }

    #[test]
    fn invalid_algorithm_falls_back_to_one() {
        let (peak_one, _) = run_algorithm_for_samples(1, 256);
        let (peak_zero, _) = run_algorithm_for_samples(0, 256);
        let (peak_huge, _) = run_algorithm_for_samples(99, 256);
        // Fallback should produce a similar-shaped signal to algorithm 1.
        assert!((peak_zero - peak_one).abs() < 0.5);
        assert!((peak_huge - peak_one).abs() < 0.5);
    }

    // -----------------------------------------------------------------------
    // get_algorithm_info coverage
    // -----------------------------------------------------------------------

    #[test]
    fn every_algorithm_info_is_self_consistent() {
        for alg in 1..=32u8 {
            let info = get_algorithm_info(alg);
            assert!(!info.carriers.is_empty(), "alg {alg} has no carriers");
            for &c in &info.carriers {
                assert!((1..=6).contains(&c), "alg {alg} has invalid carrier {c}");
            }
            for (from, to) in &info.connections {
                assert!((1..=6).contains(from), "alg {alg} connection from {from}");
                assert!((1..=6).contains(to), "alg {alg} connection to {to}");
            }
            assert!(
                info.feedback_op <= 6,
                "alg {alg} bad feedback_op {}",
                info.feedback_op
            );
        }
    }

    #[test]
    fn invalid_algorithm_info_falls_back_to_one() {
        let one = get_algorithm_info(1);
        let invalid = get_algorithm_info(0);
        assert_eq!(one.carriers, invalid.carriers);
        assert_eq!(one.feedback_op, invalid.feedback_op);
    }

    #[test]
    fn algorithm_5_has_three_carriers() {
        let info = get_algorithm_info(5);
        assert_eq!(info.carriers, vec![1, 3, 5]);
    }

    #[test]
    fn algorithm_32_has_all_carriers() {
        let info = get_algorithm_info(32);
        assert_eq!(info.carriers, vec![1, 2, 3, 4, 5, 6]);
        assert!(info.connections.is_empty());
    }

    // -----------------------------------------------------------------------
    // Algorithm naming
    // -----------------------------------------------------------------------

    #[test]
    fn every_algorithm_has_a_name() {
        for alg in 1..=32u8 {
            let name = get_algorithm_name(alg);
            assert!(!name.is_empty(), "alg {alg} has empty name");
            assert!(
                name.starts_with(&format!("{}:", alg)),
                "alg {alg} name '{name}' should start with the number"
            );
        }
    }

    #[test]
    fn invalid_algorithm_name_falls_back_to_one() {
        assert_eq!(get_algorithm_name(0), get_algorithm_name(1));
        assert_eq!(get_algorithm_name(99), get_algorithm_name(1));
    }

    // -----------------------------------------------------------------------
    // Cross-feedback paths (algorithms 4 and 6)
    // -----------------------------------------------------------------------

    #[test]
    fn algorithm_4_uses_cross_feedback_when_op4_has_feedback() {
        // Trigger an op stack and engage Op4 cross feedback.
        let mut ops_no_fb = triggered_ops();
        let mut ops_fb = triggered_ops();
        ops_fb[3].feedback = 7.0;
        // Warm up
        for _ in 0..2048 {
            process_algorithm(4, &mut ops_no_fb);
            process_algorithm(4, &mut ops_fb);
        }
        let mut diff = 0;
        for _ in 0..2048 {
            let a = process_algorithm(4, &mut ops_no_fb);
            let b = process_algorithm(4, &mut ops_fb);
            if (a - b).abs() > 1e-3 {
                diff += 1;
            }
        }
        assert!(
            diff > 100,
            "cross feedback should change the signal ({diff} differing)"
        );
    }

    #[test]
    fn algorithm_6_uses_cross_feedback_when_op6_has_feedback() {
        let mut ops_no_fb = triggered_ops();
        let mut ops_fb = triggered_ops();
        ops_fb[5].feedback = 7.0;
        for _ in 0..2048 {
            process_algorithm(6, &mut ops_no_fb);
            process_algorithm(6, &mut ops_fb);
        }
        let mut diff = 0;
        for _ in 0..2048 {
            let a = process_algorithm(6, &mut ops_no_fb);
            let b = process_algorithm(6, &mut ops_fb);
            if (a - b).abs() > 1e-3 {
                diff += 1;
            }
        }
        assert!(diff > 100, "alg 6 cross feedback should differ ({diff})");
    }
}
