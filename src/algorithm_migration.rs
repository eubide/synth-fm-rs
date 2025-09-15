use crate::algorithm_matrix::{AlgorithmMatrix, AlgorithmLibrary};

/// Migration tool to convert JSON algorithms to matrix format
pub struct AlgorithmMigrator;

impl AlgorithmMigrator {
    
    /// Create a complete library with all 32 DX7 algorithms
    pub fn create_full_dx7_library() -> AlgorithmLibrary {
        let mut library = AlgorithmLibrary::new();
        
        // Define all 32 DX7 algorithms in matrix form
        // These are the authentic DX7 algorithm configurations
        
        let algorithms = vec![
            (1, "Two Stacks", vec![1, 3], vec![(2,1), (4,3), (5,4), (6,5), (6,6)]),
            (2, "Stack + Self", vec![1, 3], vec![(2,1), (4,3), (5,4), (6,5), (2,2)]),
            (3, "Dual Stacks", vec![1, 4], vec![(2,1), (3,2), (5,4), (6,5), (6,6)]),
            (4, "Stack Loop", vec![1, 4], vec![(2,1), (3,2), (5,4), (6,5), (4,6)]),
            (5, "Three Carriers", vec![1, 3, 5], vec![(2,1), (4,3), (6,5), (6,6)]),
            (6, "Three + Feedback", vec![1, 3, 5], vec![(2,1), (4,3), (6,5), (5,6)]),
            (7, "Wide Stack", vec![1], vec![(2,1), (3,2), (4,3), (5,3), (6,5), (6,6)]),
            (8, "Double Feedback", vec![1], vec![(2,1), (3,2), (4,3), (5,3), (6,5), (4,4)]),
            (9, "Dual Branch", vec![1], vec![(2,1), (3,1), (4,3), (5,4), (6,5), (6,6)]),
            (10, "Split Branch", vec![1], vec![(2,1), (3,1), (4,3), (5,4), (6,4), (5,5)]),
            (11, "Three Mod", vec![1], vec![(2,1), (3,1), (4,1), (5,4), (6,5), (6,6)]),
            (12, "Three + Loop", vec![1], vec![(2,1), (3,1), (4,1), (5,4), (6,5), (3,6)]),
            (13, "All to One", vec![1], vec![(2,1), (3,1), (4,1), (5,1), (6,1), (6,6)]),
            (14, "Dual + Three", vec![1, 4], vec![(2,1), (3,1), (5,4), (6,4), (6,6)]),
            (15, "Branch + Self", vec![1], vec![(2,1), (3,2), (4,2), (5,4), (6,5), (2,2)]),
            (16, "Complex Tree", vec![1], vec![(2,1), (3,2), (4,2), (5,2), (6,5), (6,6)]),
            (17, "Split + Self", vec![1], vec![(2,1), (3,2), (4,2), (5,1), (6,5), (2,2)]),
            (18, "All Carriers", vec![1, 2, 3, 4, 5, 6], vec![(6,6)]),
            (19, "Triple + Tree", vec![1, 2, 3], vec![(4,1), (5,2), (6,3), (6,6)]),
            (20, "Three Pairs", vec![1, 2], vec![(3,1), (4,1), (5,2), (6,2), (3,3)]),
            (21, "Two + Four", vec![1, 2], vec![(3,1), (4,2), (5,2), (6,2), (4,4)]),
            (22, "Four Carriers", vec![1, 2, 3, 4], vec![(5,1), (6,2), (6,6)]),
            (23, "Four + Dual", vec![1, 2, 3, 4], vec![(5,3), (6,4), (5,5)]),
            (24, "Five Carriers", vec![1, 2, 3, 4, 5], vec![(6,3), (6,6)]),
            (25, "Five Simple", vec![1, 2, 3, 4, 5], vec![(6,5), (6,6)]),
            (26, "Three Two-Op", vec![1, 3, 5], vec![(2,1), (4,3), (6,5), (4,4)]),
            (27, "One Four-Op", vec![1, 2], vec![(3,2), (4,3), (5,4), (6,5), (3,3)]),
            (28, "Bass Heavy", vec![1, 4], vec![(2,1), (3,1), (5,4), (6,4), (6,6)]),
            (29, "Wide Three", vec![1, 2, 5], vec![(3,1), (4,2), (6,5), (5,5)]),
            (30, "Saw-like", vec![1, 2, 5], vec![(3,1), (4,2), (6,5), (4,4)]),
            (31, "Organ", vec![1], vec![(2,1), (3,1), (4,2), (5,2), (6,1), (6,6)]),
            (32, "Full Stack", vec![1], vec![(2,1), (3,2), (4,3), (5,4), (6,5), (6,6)]),
            
            // 🎵 ALGORITMOS PERSONALIZADOS
            (33, "Dual Feedback", vec![1, 4], vec![(2,1), (3,2), (5,4), (6,5), (2,2), (5,5)]),
            (34, "Ring Mod", vec![1], vec![(2,1), (3,1), (4,2), (5,3), (6,4)]),
            (35, "Chaos FM", vec![1, 2], vec![(3,1), (4,2), (5,3), (6,4), (1,6), (2,5)]),
        ];
        
        for (num, name, carriers, connections) in algorithms {
            let mut matrix = AlgorithmMatrix::new(num, format!("{}: {}", num, name));
            
            // Set carriers
            for carrier in carriers {
                matrix.set_carrier((carrier - 1) as usize, true);
            }
            
            // Set connections
            for (from, to) in connections {
                let from_idx = (from - 1) as usize;
                let to_idx = (to - 1) as usize;
                let amount = if from_idx == to_idx { 0.7 } else { 1.0 };
                matrix.set_connection(from_idx, to_idx, amount);
            }
            
            library.set(matrix);
        }
        
        library
    }
}