use crate::operator::Operator;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AlgorithmGraph {
    pub operators: Vec<OperatorNode>,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone)]
pub struct OperatorNode {
    pub id: u8,
    pub position: (f32, f32),
    pub is_carrier: bool,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub from: u8,
    pub to: u8,
    pub is_feedback: bool,
}

/// Simplified algorithm system using a modulation matrix
/// Each algorithm is represented as a 6x6 matrix where:
/// - matrix[from][to] = modulation amount (0.0 to 1.0)
/// - carriers[i] = true if operator i is a carrier (outputs to audio)
#[derive(Debug, Clone)]
pub struct AlgorithmMatrix {
    /// 6x6 modulation matrix: connections[from][to] = modulation_amount
    pub connections: [[f32; 6]; 6],
    /// Which operators are carriers (output to audio)
    pub carriers: [bool; 6],
    /// Algorithm number for compatibility
    pub algorithm_number: u8,
    /// Algorithm name for display
    pub name: String,
}

impl Default for AlgorithmMatrix {
    fn default() -> Self {
        // Default algorithm: all operators are carriers, no modulation
        Self {
            connections: [[0.0; 6]; 6],
            carriers: [true; 6],
            algorithm_number: 0,
            name: "Default (All Carriers)".to_string(),
        }
    }
}

impl AlgorithmMatrix {
    /// Create a new empty algorithm matrix
    pub fn new(algorithm_number: u8, name: String) -> Self {
        Self {
            connections: [[0.0; 6]; 6],
            carriers: [false; 6],
            algorithm_number,
            name,
        }
    }

    /// Process operators through the modulation matrix
    /// This is the core synthesis function - simple and efficient
    pub fn process(&self, ops: &mut [Operator; 6]) -> f32 {
        let mut outputs = [0.0_f32; 6];
        let mut modulations = [0.0_f32; 6];
        
        // First pass: collect all modulations for each operator
        // This handles feedback naturally without special cases
        for target in 0..6 {
            modulations[target] = 0.0;
            for source in 0..6 {
                if self.connections[source][target] > 0.0 {
                    // For self-feedback, use previous output
                    let source_output = if source == target {
                        ops[source].get_feedback_output()
                    } else {
                        outputs[source]
                    };
                    modulations[target] += source_output * self.connections[source][target];
                }
            }
        }
        
        // Second pass: process all operators with their modulations
        for i in 0..6 {
            outputs[i] = ops[i].process(modulations[i]);
        }
        
        // Sum all carrier outputs
        let mut total = 0.0;
        let mut carrier_count = 0;
        for i in 0..6 {
            if self.carriers[i] {
                total += outputs[i];
                carrier_count += 1;
            }
        }
        
        // Scale output based on number of carriers
        if carrier_count > 1 {
            total / (carrier_count as f32).sqrt()
        } else {
            total
        }
    }

    /// Set a connection in the matrix
    pub fn set_connection(&mut self, from: usize, to: usize, amount: f32) {
        if from < 6 && to < 6 {
            self.connections[from][to] = amount.clamp(0.0, 1.0);
        }
    }

    /// Set an operator as a carrier
    pub fn set_carrier(&mut self, operator: usize, is_carrier: bool) {
        if operator < 6 {
            self.carriers[operator] = is_carrier;
        }
    }

    /// Check if there's a connection between two operators
    pub fn has_connection(&self, from: usize, to: usize) -> bool {
        from < 6 && to < 6 && self.connections[from][to] > 0.0
    }

    /// Get all connections for visualization
    pub fn get_connections(&self) -> Vec<(usize, usize, f32)> {
        let mut connections = Vec::new();
        for from in 0..6 {
            for to in 0..6 {
                if self.connections[from][to] > 0.0 {
                    connections.push((from, to, self.connections[from][to]));
                }
            }
        }
        connections
    }
    
    /// Create an algorithm graph for GUI visualization
    pub fn create_algorithm_graph(&self) -> AlgorithmGraph {
        let mut operators = Vec::new();
        for i in 0..6 {
            operators.push(OperatorNode {
                id: (i + 1) as u8, // 1-based indexing for display
                position: (0.0, 0.0), // Will be calculated by layout
                is_carrier: self.carriers[i],
            });
        }
        
        let mut connections = Vec::new();
        for from in 0..6 {
            for to in 0..6 {
                if self.connections[from][to] > 0.0 {
                    connections.push(Connection {
                        from: (from + 1) as u8, // 1-based indexing
                        to: (to + 1) as u8,
                        is_feedback: from == to,
                    });
                }
            }
        }
        
        AlgorithmGraph {
            operators,
            connections,
        }
    }
    
    /// Calculate layout for GUI display
    pub fn calculate_layout(mut graph: AlgorithmGraph, canvas_size: (f32, f32)) -> AlgorithmGraph {
        let (width, height) = canvas_size;
        let row_spacing = 35.0;
        let column_spacing = 45.0;
        
        // Simple layout: carriers at bottom, modulators above
        let mut carrier_positions = Vec::new();
        let mut modulator_positions = Vec::new();
        
        for (i, op) in graph.operators.iter().enumerate() {
            if op.is_carrier {
                carrier_positions.push(i);
            } else {
                modulator_positions.push(i);
            }
        }
        
        // Position carriers at bottom
        for (idx, &op_idx) in carrier_positions.iter().enumerate() {
            let x = (width / 2.0) + (idx as f32 - (carrier_positions.len() as f32 - 1.0) / 2.0) * column_spacing;
            let y = height - 40.0; // Bottom row
            graph.operators[op_idx].position = (x, y);
        }
        
        // Position modulators above carriers
        for (idx, &op_idx) in modulator_positions.iter().enumerate() {
            let x = (width / 2.0) + (idx as f32 - (modulator_positions.len() as f32 - 1.0) / 2.0) * column_spacing;
            let y = height - 80.0 - (idx / 3) as f32 * row_spacing; // Multiple rows if needed
            graph.operators[op_idx].position = (x, y);
        }
        
        graph
    }
}

/// Algorithm library with all DX7 algorithms converted to matrix form
pub struct AlgorithmLibrary {
    algorithms: HashMap<u8, AlgorithmMatrix>,
}

impl AlgorithmLibrary {
    /// Create a new algorithm library with all DX7 algorithms
    pub fn new() -> Self {
        let mut library = Self {
            algorithms: HashMap::new(),
        };
        library.init_dx7_algorithms();
        library
    }

    /// Get an algorithm by number
    pub fn get(&self, algorithm_number: u8) -> Option<&AlgorithmMatrix> {
        self.algorithms.get(&algorithm_number)
    }

    /// Get a mutable algorithm by number
    pub fn get_mut(&mut self, algorithm_number: u8) -> Option<&mut AlgorithmMatrix> {
        self.algorithms.get_mut(&algorithm_number)
    }

    /// Add or update an algorithm
    pub fn set(&mut self, algorithm: AlgorithmMatrix) {
        self.algorithms.insert(algorithm.algorithm_number, algorithm);
    }

    /// Get all algorithms sorted by number
    pub fn get_all(&self) -> Vec<&AlgorithmMatrix> {
        let mut algorithms: Vec<_> = self.algorithms.values().collect();
        algorithms.sort_by_key(|a| a.algorithm_number);
        algorithms
    }

    /// Initialize with DX7 algorithms
    fn init_dx7_algorithms(&mut self) {
        // Algorithm 1: Two Stacks
        let mut alg1 = AlgorithmMatrix::new(1, "1: Two Stacks".to_string());
        alg1.set_carrier(0, true);  // Op1 is carrier
        alg1.set_carrier(2, true);  // Op3 is carrier
        alg1.set_connection(1, 0, 1.0);  // Op2 -> Op1
        alg1.set_connection(3, 2, 1.0);  // Op4 -> Op3
        alg1.set_connection(4, 3, 1.0);  // Op5 -> Op4
        alg1.set_connection(5, 4, 1.0);  // Op6 -> Op5
        alg1.set_connection(5, 5, 0.7);  // Op6 feedback
        self.set(alg1);

        // Algorithm 2: Stack + Self
        let mut alg2 = AlgorithmMatrix::new(2, "2: Stack + Self".to_string());
        alg2.set_carrier(0, true);  // Op1 is carrier
        alg2.set_carrier(2, true);  // Op3 is carrier
        alg2.set_connection(1, 0, 1.0);  // Op2 -> Op1
        alg2.set_connection(3, 2, 1.0);  // Op4 -> Op3
        alg2.set_connection(4, 3, 1.0);  // Op5 -> Op4
        alg2.set_connection(5, 4, 1.0);  // Op6 -> Op5
        alg2.set_connection(1, 1, 0.7);  // Op2 feedback
        self.set(alg2);

        // Algorithm 3: Dual Stacks
        let mut alg3 = AlgorithmMatrix::new(3, "3: Dual Stacks".to_string());
        alg3.set_carrier(0, true);  // Op1 is carrier
        alg3.set_carrier(3, true);  // Op4 is carrier
        alg3.set_connection(1, 0, 1.0);  // Op2 -> Op1
        alg3.set_connection(2, 1, 1.0);  // Op3 -> Op2
        alg3.set_connection(4, 3, 1.0);  // Op5 -> Op4
        alg3.set_connection(5, 4, 1.0);  // Op6 -> Op5
        alg3.set_connection(5, 5, 0.7);  // Op6 feedback
        self.set(alg3);

        // Algorithm 4: Stack Loop
        let mut alg4 = AlgorithmMatrix::new(4, "4: Stack Loop".to_string());
        alg4.set_carrier(0, true);  // Op1 is carrier
        alg4.set_carrier(3, true);  // Op4 is carrier
        alg4.set_connection(1, 0, 1.0);  // Op2 -> Op1
        alg4.set_connection(2, 1, 1.0);  // Op3 -> Op2
        alg4.set_connection(4, 3, 1.0);  // Op5 -> Op4
        alg4.set_connection(5, 4, 1.0);  // Op6 -> Op5
        alg4.set_connection(3, 5, 0.5);  // Op4 -> Op6 (feedback loop)
        self.set(alg4);

        // Algorithm 5: Classic E.Piano
        let mut alg5 = AlgorithmMatrix::new(5, "5: Classic E.Piano".to_string());
        alg5.set_carrier(0, true);  // Op1 is carrier
        alg5.set_carrier(2, true);  // Op3 is carrier
        alg5.set_carrier(4, true);  // Op5 is carrier
        alg5.set_connection(1, 0, 1.0);  // Op2 -> Op1
        alg5.set_connection(3, 2, 1.0);  // Op4 -> Op3
        alg5.set_connection(5, 4, 1.0);  // Op6 -> Op5
        alg5.set_connection(5, 5, 0.7);  // Op6 feedback
        self.set(alg5);

        // We'll add more algorithms as needed
        // This demonstrates the pattern for converting from JSON to matrix form
    }
}

/// Convert legacy algorithm definition to matrix form
pub fn convert_legacy_algorithm(
    algorithm_number: u8,
    name: String,
    carriers: &[u8],
    connections: &[(u8, u8)],
) -> AlgorithmMatrix {
    let mut matrix = AlgorithmMatrix::new(algorithm_number, name);
    
    // Set carriers (convert from 1-based to 0-based indexing)
    for &carrier in carriers {
        if carrier > 0 && carrier <= 6 {
            matrix.set_carrier((carrier - 1) as usize, true);
        }
    }
    
    // Set connections (convert from 1-based to 0-based indexing)
    for &(from, to) in connections {
        if from > 0 && from <= 6 && to > 0 && to <= 6 {
            let from_idx = (from - 1) as usize;
            let to_idx = (to - 1) as usize;
            
            // Self-feedback gets a different weight
            let amount = if from_idx == to_idx { 0.7 } else { 1.0 };
            matrix.set_connection(from_idx, to_idx, amount);
        }
    }
    
    matrix
}

/// Helper function to create custom algorithms easily
pub fn create_custom_algorithm(
    number: u8,
    name: &str,
    carriers: &[u8],        // Which operators output to audio (1-6)
    connections: &[(u8, u8)], // (from, to) pairs for modulation
    feedback_amounts: &[(u8, f32)], // (operator, amount) for custom feedback
) -> AlgorithmMatrix {
    let mut matrix = AlgorithmMatrix::new(number, format!("{}: {}", number, name));
    
    // Set carriers
    for &carrier in carriers {
        if carrier >= 1 && carrier <= 6 {
            matrix.set_carrier((carrier - 1) as usize, true);
        }
    }
    
    // Set connections
    for &(from, to) in connections {
        if from >= 1 && from <= 6 && to >= 1 && to <= 6 {
            let from_idx = (from - 1) as usize;
            let to_idx = (to - 1) as usize;
            let amount = if from_idx == to_idx { 0.7 } else { 1.0 };
            matrix.set_connection(from_idx, to_idx, amount);
        }
    }
    
    // Set custom feedback amounts
    for &(op, amount) in feedback_amounts {
        if op >= 1 && op <= 6 {
            let op_idx = (op - 1) as usize;
            matrix.set_connection(op_idx, op_idx, amount);
        }
    }
    
    matrix
}