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
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.7
}

/// Algorithm 2: Stack + Self
/// Carriers: [1, 3] - Connections: [(2,1), (4,3), (5,4), (6,5), (2,2)]
fn algorithm_2(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op2 -> Op1 (with Op2 feedback)
    let op2_out = ops[1].process(ops[1].get_feedback_output() * 0.7);
    let op1_out = ops[0].process(op2_out);

    // Stack 2: Op6 -> Op5 -> Op4 -> Op3
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    (op1_out + op3_out) * 0.7
}

/// Algorithm 3: Dual Stacks
/// Carriers: [1, 4] - Connections: [(2,1), (3,2), (5,4), (6,5), (6,6)]
fn algorithm_3(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op3 -> Op2 -> Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Stack 2: Op6 -> Op5 -> Op4 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    (op1_out + op4_out) * 0.7
}

/// Algorithm 4: Stack Loop
/// Carriers: [1, 4] - Connections: [(2,1), (3,2), (5,4), (6,5), (4,6)]
fn algorithm_4(ops: &mut [Operator; 6]) -> f32 {
    // Stack 1: Op3 -> Op2 -> Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    // Stack 2 with loop: Op6 -> Op5 -> Op4, Op4 -> Op6
    let op6_out = ops[5].process(ops[3].get_feedback_output() * 0.5);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    (op1_out + op4_out) * 0.7
}

/// Algorithm 5: Three Carriers (Classic E.Piano)
/// Carriers: [1, 3, 5] - Connections: [(2,1), (4,3), (6,5), (6,6)]
fn algorithm_5(ops: &mut [Operator; 6]) -> f32 {
    // Pair 1: Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Pair 2: Op4 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Pair 3: Op6 -> Op5 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);

    (op1_out + op3_out + op5_out) / 1.7
}

/// Algorithm 6: Three + Feedback
/// Carriers: [1, 3, 5] - Connections: [(2,1), (4,3), (6,5), (5,6)]
fn algorithm_6(ops: &mut [Operator; 6]) -> f32 {
    // Pair 1: Op2 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out);

    // Pair 2: Op4 -> Op3
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out);

    // Pair 3: Op6 -> Op5, Op5 -> Op6 (cross feedback)
    let op6_out = ops[5].process(ops[4].get_feedback_output() * 0.5);
    let op5_out = ops[4].process(op6_out);

    (op1_out + op3_out + op5_out) / 1.7
}

/// Algorithm 7: Wide Stack
/// Carriers: [1] - Connections: [(2,1), (3,2), (4,3), (5,3), (6,5), (6,6)]
fn algorithm_7(ops: &mut [Operator; 6]) -> f32 {
    // Branch: Op6 -> Op5 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);

    // Main stack: Op4, Op5 -> Op3 -> Op2 -> Op1
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op4_out + op5_out);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    op1_out
}

/// Algorithm 8: Double Feedback
/// Carriers: [1] - Connections: [(2,1), (3,2), (4,3), (5,3), (6,5), (4,4)]
fn algorithm_8(ops: &mut [Operator; 6]) -> f32 {
    // Branch: Op6 -> Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Main stack: Op4, Op5 -> Op3 -> Op2 -> Op1 (with Op4 feedback)
    let op4_out = ops[3].process(ops[3].get_feedback_output() * 0.7);
    let op3_out = ops[2].process(op4_out + op5_out);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    op1_out
}

/// Algorithm 9: Dual Branch
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,3), (5,4), (6,5), (6,6)]
fn algorithm_9(ops: &mut [Operator; 6]) -> f32 {
    // Branch 1: Op6 -> Op5 -> Op4 -> Op3 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);

    // Branch 2: Op2 -> Op1, Op3 -> Op1
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out);

    op1_out
}

/// Algorithm 10: Split Branch
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,3), (5,4), (6,4), (5,5)]
fn algorithm_10(ops: &mut [Operator; 6]) -> f32 {
    // Op5 with feedback
    let op5_out = ops[4].process(ops[4].get_feedback_output() * 0.7);

    // Op6 -> Op4, Op5 -> Op4
    let op6_out = ops[5].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    // Op4 -> Op3 -> Op1, Op2 -> Op1
    let op3_out = ops[2].process(op4_out);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out);

    op1_out
}

/// Algorithm 11: Three Mod
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,1), (5,4), (6,5), (6,6)]
fn algorithm_11(ops: &mut [Operator; 6]) -> f32 {
    // Branch: Op6 -> Op5 -> Op4 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    // Multiple modulators to Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out + op4_out);

    op1_out
}

/// Algorithm 12: Three + Loop
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,1), (5,4), (6,5), (3,6)]
fn algorithm_12(ops: &mut [Operator; 6]) -> f32 {
    // Op3 -> Op6 -> Op5 -> Op4 (with Op3 feedback from Op6)
    let op6_out = ops[5].process(ops[2].get_feedback_output() * 0.5);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    // Multiple modulators to Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out + op4_out);

    op1_out
}

/// Algorithm 13: All to One
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,1), (5,1), (6,1), (6,6)]
fn algorithm_13(ops: &mut [Operator; 6]) -> f32 {
    // All operators modulate Op1 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out + op4_out + op5_out + op6_out);

    op1_out
}

/// Algorithm 14: Dual + Three
/// Carriers: [1, 4] - Connections: [(2,1), (3,1), (5,4), (6,4), (6,6)]
fn algorithm_14(ops: &mut [Operator; 6]) -> f32 {
    // Multiple modulators to Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out);

    // Multiple modulators to Op4 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    (op1_out + op4_out) * 0.7
}

/// Algorithm 15: Branch + Self
/// Carriers: [1] - Connections: [(2,1), (3,2), (4,2), (5,4), (6,5), (2,2)]
fn algorithm_15(ops: &mut [Operator; 6]) -> f32 {
    // Branch: Op6 -> Op5 -> Op4
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);

    // Op2 with feedback, multiple modulators
    let op2_out = ops[1].process(ops[1].get_feedback_output() * 0.7 + op4_out);
    let _op3_out = ops[2].process(op2_out);
    let op1_out = ops[0].process(op2_out);

    op1_out
}

/// Algorithm 16: Complex Tree
/// Carriers: [1] - Connections: [(2,1), (3,2), (4,2), (5,2), (6,5), (6,6)]
fn algorithm_16(ops: &mut [Operator; 6]) -> f32 {
    // Branch: Op6 -> Op5 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);

    // Multiple modulators to Op2
    let op4_out = ops[3].process(0.0);
    let op2_out = ops[1].process(op4_out + op5_out);

    // Op3 -> Op2, Op2 -> Op1
    let _op3_out = ops[2].process(op2_out);
    let op1_out = ops[0].process(op2_out);

    op1_out
}

/// Algorithm 17: Split + Self
/// Carriers: [1] - Connections: [(2,1), (3,2), (4,2), (5,1), (6,5), (2,2)]
fn algorithm_17(ops: &mut [Operator; 6]) -> f32 {
    // Branch: Op6 -> Op5
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);

    // Op2 with feedback and modulators
    let op4_out = ops[3].process(0.0);
    let op2_out = ops[1].process(ops[1].get_feedback_output() * 0.7 + op4_out);

    // Convergence at Op1
    let _op3_out = ops[2].process(op2_out);
    let op1_out = ops[0].process(op2_out + op5_out);

    op1_out
}

/// Algorithm 18: All Carriers
/// Carriers: [1, 2, 3, 4, 5, 6] - Connections: [(6,6)]
fn algorithm_18(ops: &mut [Operator; 6]) -> f32 {
    // All operators are carriers (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out + op6_out) / 2.4
}

/// Algorithm 19: Triple + Tree
/// Carriers: [1, 2, 3] - Connections: [(4,1), (5,2), (6,3), (6,6)]
fn algorithm_19(ops: &mut [Operator; 6]) -> f32 {
    // Modulators to carriers (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);

    let op3_out = ops[2].process(op6_out);
    let op2_out = ops[1].process(op5_out);
    let op1_out = ops[0].process(op4_out);

    (op1_out + op2_out + op3_out) / 1.7
}

/// Algorithm 20: Three Pairs
/// Carriers: [1, 2] - Connections: [(3,1), (4,1), (5,2), (6,2), (3,3)]
fn algorithm_20(ops: &mut [Operator; 6]) -> f32 {
    // Op3 with feedback
    let op3_out = ops[2].process(ops[2].get_feedback_output() * 0.7);

    // Multiple modulators to carriers
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);

    let op2_out = ops[1].process(op5_out + op6_out);
    let op1_out = ops[0].process(op3_out + op4_out);

    (op1_out + op2_out) * 0.7
}

/// Algorithm 21: Two + Four
/// Carriers: [1, 2] - Connections: [(3,1), (4,2), (5,2), (6,2), (4,4)]
fn algorithm_21(ops: &mut [Operator; 6]) -> f32 {
    // Op4 with feedback
    let op4_out = ops[3].process(ops[3].get_feedback_output() * 0.7);

    // Modulators
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(0.0);
    let op3_out = ops[2].process(0.0);

    let op2_out = ops[1].process(op4_out + op5_out + op6_out);
    let op1_out = ops[0].process(op3_out);

    (op1_out + op2_out) * 0.7
}

/// Algorithm 22: Four Carriers
/// Carriers: [1, 2, 3, 4] - Connections: [(5,1), (6,2), (6,6)]
fn algorithm_22(ops: &mut [Operator; 6]) -> f32 {
    // Modulators (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);

    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(op6_out);
    let op1_out = ops[0].process(op5_out);

    (op1_out + op2_out + op3_out + op4_out) / 2.0
}

/// Algorithm 23: Four + Dual
/// Carriers: [1, 2, 3, 4] - Connections: [(5,3), (6,4), (5,5)]
fn algorithm_23(ops: &mut [Operator; 6]) -> f32 {
    // Op5 with feedback
    let op5_out = ops[4].process(ops[4].get_feedback_output() * 0.7);
    let op6_out = ops[5].process(0.0);

    let op4_out = ops[3].process(op6_out);
    let op3_out = ops[2].process(op5_out);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out) / 2.0
}

/// Algorithm 24: Five Carriers
/// Carriers: [1, 2, 3, 4, 5] - Connections: [(6,3), (6,6)]
fn algorithm_24(ops: &mut [Operator; 6]) -> f32 {
    // Op6 with feedback, modulates Op3
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);

    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(op6_out);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out) / 2.2
}

/// Algorithm 25: Five Simple
/// Carriers: [1, 2, 3, 4, 5] - Connections: [(6,5), (6,6)]
fn algorithm_25(ops: &mut [Operator; 6]) -> f32 {
    // Op6 with feedback, modulates Op5
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);

    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out + op3_out + op4_out + op5_out) / 2.2
}

/// Algorithm 26: Three Two-Op
/// Carriers: [1, 3, 5] - Connections: [(2,1), (4,3), (6,5), (4,4)]
fn algorithm_26(ops: &mut [Operator; 6]) -> f32 {
    // Three pairs with Op4 feedback
    let op4_out = ops[3].process(ops[3].get_feedback_output() * 0.7);
    let op6_out = ops[5].process(0.0);
    let op2_out = ops[1].process(0.0);

    let op5_out = ops[4].process(op6_out);
    let op3_out = ops[2].process(op4_out);
    let op1_out = ops[0].process(op2_out);

    (op1_out + op3_out + op5_out) / 1.7
}

/// Algorithm 27: One Four-Op
/// Carriers: [1, 2] - Connections: [(3,2), (4,3), (5,4), (6,5), (3,3)]
fn algorithm_27(ops: &mut [Operator; 6]) -> f32 {
    // Four-op stack: Op6 -> Op5 -> Op4 -> Op3 (with Op3 feedback)
    let op6_out = ops[5].process(0.0);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(ops[2].get_feedback_output() * 0.7 + op4_out);

    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(0.0);

    (op1_out + op2_out) * 0.7
}

/// Algorithm 28: Bass Heavy
/// Carriers: [1, 4] - Connections: [(2,1), (3,1), (5,4), (6,4), (6,6)]
fn algorithm_28(ops: &mut [Operator; 6]) -> f32 {
    // Multiple modulators to Op1
    let op3_out = ops[2].process(0.0);
    let op2_out = ops[1].process(0.0);
    let op1_out = ops[0].process(op2_out + op3_out);

    // Multiple modulators to Op4 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(op5_out + op6_out);

    (op1_out + op4_out) * 0.7
}

/// Algorithm 29: Wide Three
/// Carriers: [1, 2, 5] - Connections: [(3,1), (4,2), (6,5), (5,5)]
fn algorithm_29(ops: &mut [Operator; 6]) -> f32 {
    // Op5 with feedback
    let _op5_out = ops[4].process(ops[4].get_feedback_output() * 0.7);

    // Modulators to carriers
    let op6_out = ops[5].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);

    let op5_carrier = ops[4].process(op6_out);
    let op2_out = ops[1].process(op4_out);
    let op1_out = ops[0].process(op3_out);

    (op1_out + op2_out + op5_carrier) / 1.7
}

/// Algorithm 30: Saw-like
/// Carriers: [1, 2, 5] - Connections: [(3,1), (4,2), (6,5), (4,4)]
fn algorithm_30(ops: &mut [Operator; 6]) -> f32 {
    // Op4 with feedback
    let op4_out = ops[3].process(ops[3].get_feedback_output() * 0.7);

    // Modulators to carriers
    let op6_out = ops[5].process(0.0);
    let op3_out = ops[2].process(0.0);

    let op5_out = ops[4].process(op6_out);
    let op2_out = ops[1].process(op4_out);
    let op1_out = ops[0].process(op3_out);

    (op1_out + op2_out + op5_out) / 1.7
}

/// Algorithm 31: Organ
/// Carriers: [1] - Connections: [(2,1), (3,1), (4,2), (5,2), (6,1), (6,6)]
fn algorithm_31(ops: &mut [Operator; 6]) -> f32 {
    // Complex modulation network (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(0.0);
    let op4_out = ops[3].process(0.0);
    let op3_out = ops[2].process(0.0);

    // Op2 receives multiple modulators
    let op2_out = ops[1].process(op4_out + op5_out);

    // Op1 receives multiple modulators
    let op1_out = ops[0].process(op2_out + op3_out + op6_out);

    op1_out
}

/// Algorithm 32: Full Stack
/// Carriers: [1] - Connections: [(2,1), (3,2), (4,3), (5,4), (6,5), (6,6)]
fn algorithm_32(ops: &mut [Operator; 6]) -> f32 {
    // Full serial chain: Op6 -> Op5 -> Op4 -> Op3 -> Op2 -> Op1 (with Op6 feedback)
    let op6_out = ops[5].process(ops[5].get_feedback_output() * 0.7);
    let op5_out = ops[4].process(op6_out);
    let op4_out = ops[3].process(op5_out);
    let op3_out = ops[2].process(op4_out);
    let op2_out = ops[1].process(op3_out);
    let op1_out = ops[0].process(op2_out);

    op1_out
}

/// Get algorithm name for display
pub fn get_algorithm_name(algorithm_number: u8) -> &'static str {
    match algorithm_number {
        1 => "1: Two Stacks",
        2 => "2: Stack + Self",
        3 => "3: Dual Stacks",
        4 => "4: Stack Loop",
        5 => "5: Three Carriers",
        6 => "6: Three + Feedback",
        7 => "7: Wide Stack",
        8 => "8: Double Feedback",
        9 => "9: Dual Branch",
        10 => "10: Split Branch",
        11 => "11: Three Mod",
        12 => "12: Three + Loop",
        13 => "13: All to One",
        14 => "14: Dual + Three",
        15 => "15: Branch + Self",
        16 => "16: Complex Tree",
        17 => "17: Split + Self",
        18 => "18: All Carriers",
        19 => "19: Triple + Tree",
        20 => "20: Three Pairs",
        21 => "21: Two + Four",
        22 => "22: Four Carriers",
        23 => "23: Four + Dual",
        24 => "24: Five Carriers",
        25 => "25: Five Simple",
        26 => "26: Three Two-Op",
        27 => "27: One Four-Op",
        28 => "28: Bass Heavy",
        29 => "29: Wide Three",
        30 => "30: Saw-like",
        31 => "31: Organ",
        32 => "32: Full Stack",
        _ => "1: Two Stacks",
    }
}