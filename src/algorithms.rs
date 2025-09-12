use crate::operator::Operator;

pub struct Algorithm {
    pub number: u8,
    pub name: String,
}

impl Algorithm {
    pub fn process_algorithm(
        algorithm: u8,
        ops: &mut [Operator; 6],
        _base_frequency: f32,
        _velocity: f32,
    ) -> f32 {
        // Don't re-trigger operators here - they should only be triggered on note_on
        
        match algorithm {
            1 => {
                // Algorithm 1: 6→5→4→3→2→1 (full stack)
                // Each operator modulates the next with appropriate scaling
                let op6 = ops[5].process(0.0) * 8.0;     // Scale modulation depth
                let op5 = ops[4].process(op6) * 8.0;
                let op4 = ops[3].process(op5) * 8.0;
                let op3 = ops[2].process(op4) * 8.0;
                let op2 = ops[1].process(op3) * 8.0;
                ops[0].process(op2)
            }
            
            2 => {
                // Algorithm 2: Two branches
                let op6 = ops[5].process(0.0) * 8.0;
                let op5 = ops[4].process(op6) * 8.0;
                let op4 = ops[3].process(op5) * 8.0;
                let op3 = ops[2].process(op4) * 8.0;
                let op2 = ops[1].process(0.0) * 8.0;
                ops[0].process(op2 + op3)
            }
            
            3 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(op5);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op3);
                ops[0].process(op2 + op4)
            }
            
            4 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op4);
                let op2 = ops[1].process(op3);
                ops[0].process(op2 + op5)
            }
            
            5 => {
                // Algorithm 5: Popular for E.Piano sounds
                let op6 = ops[5].process(0.0) * 8.0;
                let op5 = ops[4].process(0.0) * 8.0;
                let op4 = ops[3].process(0.0);  // Carrier
                let op3 = ops[2].process(op6);  // Carrier modulated by op6
                let op2 = ops[1].process(op5);  // Carrier modulated by op5
                ops[0].process((op2 + op3) * 4.0) + op4  // Main carrier gets less modulation
            }
            
            6 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op6);
                let op2 = ops[1].process(op5 + op4);
                ops[0].process(op2 + op3)
            }
            
            7 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op5);
                let op2 = ops[1].process(op4);
                let op1 = ops[0].process(op3);
                op1 + op2
            }
            
            8 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op5);
                let op2 = ops[1].process(op4);
                let op1 = ops[0].process(op2);
                op1 + op3
            }
            
            9 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op3);
                let op1 = ops[0].process(op2);
                op1 + op4 + op5
            }
            
            10 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(op6);
                let op3 = ops[2].process(op5);
                let op2 = ops[1].process(op3);
                let op1 = ops[0].process(op2);
                op1 + op4
            }
            
            11 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op4);
                let op1 = ops[0].process(op3);
                op1 + op2 + op5
            }
            
            12 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(op5);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op4);
                let op1 = ops[0].process(op3);
                op1 + op2
            }
            
            13 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(op6);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op5);
                let op1 = ops[0].process(op3);
                op1 + op2 + op4
            }
            
            14 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op5 + op4);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(op3);
                op1 + op2
            }
            
            15 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op5);
                let op1 = ops[0].process(op4);
                op1 + op2 + op3
            }
            
            16 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(op6);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op5);
                let op1 = ops[0].process(op4);
                op1 + op2 + op3
            }
            
            17 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op5 + op6);
                let op2 = ops[1].process(op4);
                let op1 = ops[0].process(op3);
                op1 + op2
            }
            
            18 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(op5);
                let op3 = ops[2].process(op6);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(op4);
                op1 + op2 + op3
            }
            
            19 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(op6);
                let op3 = ops[2].process(op5);
                let op2 = ops[1].process(op4);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3
            }
            
            20 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(op5);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(op4);
                op1 + op2 + op3
            }
            
            21 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(op5);
                let op3 = ops[2].process(op4);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3
            }
            
            22 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(op5);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4
            }
            
            23 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op5);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4
            }
            
            24 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4 + op5
            }
            
            25 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4 + op5
            }
            
            26 | 27 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(op6);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(op5);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4
            }
            
            28 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op5);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4
            }
            
            29 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(op6);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4 + op5
            }
            
            30 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(op6);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4 + op5
            }
            
            31 => {
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                op1 + op2 + op3 + op4 + op5 + op6
            }
            
            32 => {
                // Algorithm 32: All carriers in parallel (additive synthesis)
                let op6 = ops[5].process(0.0);
                let op5 = ops[4].process(0.0);
                let op4 = ops[3].process(0.0);
                let op3 = ops[2].process(0.0);
                let op2 = ops[1].process(0.0);
                let op1 = ops[0].process(0.0);
                (op1 + op2 + op3 + op4 + op5 + op6) * 0.5  // Scale down to prevent clipping
            }
            
            _ => 0.0,
        }
    }
    
    pub fn get_all_algorithms() -> Vec<Algorithm> {
        vec![
            Algorithm { number: 1, name: "Stack 6→5→4→3→2→1".to_string() },
            Algorithm { number: 2, name: "Stack + Branch".to_string() },
            Algorithm { number: 3, name: "3 Mod + 3 Carrier".to_string() },
            Algorithm { number: 4, name: "4 Mod + 2 Carrier".to_string() },
            Algorithm { number: 5, name: "2 Stacks of 2".to_string() },
            Algorithm { number: 6, name: "3 Stacks".to_string() },
            Algorithm { number: 7, name: "Diamond + Carrier".to_string() },
            Algorithm { number: 8, name: "Two Diamonds".to_string() },
            Algorithm { number: 9, name: "Complex Mod".to_string() },
            Algorithm { number: 10, name: "3 Into 2".to_string() },
            Algorithm { number: 11, name: "Harmonic Series".to_string() },
            Algorithm { number: 12, name: "2 Feedback Loops".to_string() },
            Algorithm { number: 13, name: "3 Vertical Stacks".to_string() },
            Algorithm { number: 14, name: "Dual Feedback".to_string() },
            Algorithm { number: 15, name: "Triple Stack".to_string() },
            Algorithm { number: 16, name: "Organ Mode".to_string() },
            Algorithm { number: 17, name: "5 Mod 1 Carrier".to_string() },
            Algorithm { number: 18, name: "3 Op Feedback".to_string() },
            Algorithm { number: 19, name: "Triple Mod".to_string() },
            Algorithm { number: 20, name: "Dual Carriers".to_string() },
            Algorithm { number: 21, name: "Two Pairs".to_string() },
            Algorithm { number: 22, name: "4 Carriers".to_string() },
            Algorithm { number: 23, name: "5 Parallel".to_string() },
            Algorithm { number: 24, name: "6 Parallel".to_string() },
            Algorithm { number: 25, name: "Triple Feedback".to_string() },
            Algorithm { number: 26, name: "4 Carriers Feedback".to_string() },
            Algorithm { number: 27, name: "5 Carrier Split".to_string() },
            Algorithm { number: 28, name: "Complex Routing".to_string() },
            Algorithm { number: 29, name: "5 Additive".to_string() },
            Algorithm { number: 30, name: "5 Carriers + Mod".to_string() },
            Algorithm { number: 31, name: "6 Carriers".to_string() },
            Algorithm { number: 32, name: "6 Additive".to_string() },
        ]
    }
}