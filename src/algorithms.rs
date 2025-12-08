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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 2: Stack + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,5), (2,2)]
fn algorithm_2(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op2 -> Op1 (with Op2 feedback)
    let op2_out = ops[1].process(ops[1].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 4: Stack Loop
/// Carriers: [1, 4] - Connections: [(2,1), (3,2), (5,4), (6,5), (4,6)]
fn algorithm_4(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op3 -> Op2 -> Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Stack 2 with loop: Op6 -> Op5 -> Op4, Op4 -> Op6
    // Use Op6's feedback parameter for the feedback loop strength
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 5: Triple Output
/// Carriers: [1, 3, 4] - Connections: [(6,2), (2,1), (5,3), (6,6)]
fn algorithm_5(ops: &mut [Operator; 6]) -> f32 {
    // Op6 with feedback modulates Op2
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op2_out = ops[1].process(op6_out);

    // Op2 modulates Op1
    let op1_out = ops[0].process(op2_out);

    // Op5 modulates Op3
    let op5_out = ops[4].process(0.0);
    let op3_out = ops[2].process(op5_out);

    // Op4 is carrier (no modulation)
    let op4_out = ops[3].process(0.0);

    (op1_out + op3_out + op4_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 6: Triple Split
/// Carriers: [1, 3, 4] - Connections: [(6,2), (2,1), (5,3), (5,6)]
fn algorithm_6(ops: &mut [Operator; 6]) -> f32 {
    // Op5 generates output (no modulation input)
    let op5_out = ops[4].process(0.0);

    // Op5 modulates Op6 and Op3
    // Op6 uses feedback to control its response
    let op6_out = ops[5].process(op5_out + ops[5].get_feedback_output());
    let op3_out = ops[2].process(op5_out);

    // Op6 modulates Op2, Op2 modulates Op1
    let op2_out = ops[1].process(op6_out);
    let op1_out = ops[0].process(op2_out);

    // Op4 is carrier (no modulation)
    let op4_out = ops[3].process(0.0);

    // Only carriers contribute to output: Op1, Op3, Op4
    (op1_out + op3_out + op4_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 7: Dual + Stack
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,5), (6,6)]
fn algorithm_7(ops: &mut [Operator; 6]) -> f32 {
    // Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Stack: Op6 -> Op5 -> Op4 -> Op3 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op4_out = ops[3].process(ops[3].get_feedback_output());
    let op3_out = ops[2].process(op4_out + op5_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 9: Dual + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,5), (2,2)]
fn algorithm_9(ops: &mut [Operator; 6]) -> f32 {
    // Op2 with feedback -> Op1
    let op2_out = ops[1].process(ops[1].get_feedback_output());
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
    let op3_out = ops[2].process(ops[2].get_feedback_output());

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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    (op1_out + op4_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 12: Triple Mod
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,3), (6,3), (2,2)]
fn algorithm_12(ops: &mut [Operator; 6]) -> f32 {
    // Op2 with feedback -> Op1
    let op2_out = ops[1].process(ops[1].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.71 // √2 = 1.41, inverse = 0.71
}

/// Algorithm 15: Stack + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,4), (2,2)]
fn algorithm_15(ops: &mut [Operator; 6]) -> f32 {
    // Op2 with feedback -> Op1
    let op2_out = ops[1].process(ops[1].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(op6_out);

    // Op4 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Op2, Op3, Op5 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out + op5_out);

    op1_out
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
    let op2_out = ops[1].process(ops[1].get_feedback_output());
    let op1_out = ops[0].process(op2_out + op3_out + op5_out);

    op1_out
}

/// Algorithm 18: Quad + Stack
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,1), (5,4), (6,5), (3,3)]
fn algorithm_18(ops: &mut [Operator; 6]) -> f32 {
    // Op6 -> Op5 -> Op4
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    // Op3 with feedback
    let op3_out = ops[2].process(ops[2].get_feedback_output());

    // Op2, Op3, Op4 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out + op4_out);

    op1_out
}

/// Algorithm 19: Triple + Tree
/// Carriers: [1, 4, 5] - Connections: [(2,1), (3,1), (4,1), (5,4), (6,5), (3,3)]
fn algorithm_19(ops: &mut [Operator; 6]) -> f32 {
    // Op6 -> Op5 (carrier)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op5 -> Op4 (carrier)
    let op4_out = ops[3].process(op5_out);

    // Op3 with feedback
    let op3_out = ops[2].process(ops[2].get_feedback_output());

    // Op2, Op3, Op4 -> Op1 (carrier)
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out + op4_out);

    (op1_out + op4_out + op5_out) * 0.58 // √3 = 1.73, inverse = 0.58
}

/// Algorithm 20: Triple + Dual
/// Carriers: [1, 2, 4] - Connections: [(3,1), (3,2), (5,4), (6,4), (3,3)]
fn algorithm_20(ops: &mut [Operator; 6]) -> f32 {
    // Op3 with feedback
    let op3_out = ops[2].process(ops[2].get_feedback_output());

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
    let op3_out = ops[2].process(ops[2].get_feedback_output());

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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op3_out = ops[2].process(ops[2].get_feedback_output());
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
    let op5_out = ops[4].process(ops[4].get_feedback_output());
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
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
    let op5_out = ops[4].process(ops[4].get_feedback_output());
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    // Op1, Op2, Op6 are carriers (no modulation)
    let op6_out = ops[5].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op6_out) * 0.5 // √4 = 2.0, inverse = 0.5
}

/// Algorithm 31: Six Operators
/// Carriers: [1, 2, 3, 4, 5, 6] - Connections: [(6,6)]
fn algorithm_31(ops: &mut [Operator; 6]) -> f32 {
    // All operators are carriers (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output());
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out + op6_out) * 0.41 // √6 = 2.45, inverse = 0.41
}

/// Algorithm 32: All Carriers
/// Carriers: [1, 2, 3, 4, 5, 6] - Connections: [(6,6)]
fn algorithm_32(ops: &mut [Operator; 6]) -> f32 {
    // All operators are carriers (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output());
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
        5 => "5: Triple Output",
        6 => "6: Triple Split",
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
        19 => "19: Triple + Tree",
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
        31 => "31: Six Operators",
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
            connections: vec![(2, 1), (3, 2), (5, 4), (6, 5)],
            feedback_op: 6,
        },
        5 => AlgorithmInfo {
            carriers: vec![1, 3, 4],
            connections: vec![(2, 1), (5, 3), (6, 2)],
            feedback_op: 6,
        },
        6 => AlgorithmInfo {
            carriers: vec![1, 3, 4],
            connections: vec![(2, 1), (5, 3), (5, 6), (6, 2)],
            feedback_op: 6,
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
            connections: vec![(2, 1), (3, 1), (5, 4), (6, 5)],
            feedback_op: 3,
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
            carriers: vec![1, 2, 3, 4, 5, 6],
            connections: vec![],
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
