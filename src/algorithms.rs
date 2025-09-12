use crate::operator::Operator;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlgorithmDef {
    pub algorithm: u8,
    pub name: String,
    pub carriers: Vec<u8>,
    pub connections: Vec<ConnectionDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ConnectionDef {
    pub from: u8,
    pub to: u8,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct GridLayout {
    #[allow(dead_code)]
    pub grid: HashMap<GridPosition, u8>,
    pub operator_positions: HashMap<u8, GridPosition>,
    pub dimensions: (usize, usize), // (rows, cols)
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub from: u8,
    pub to: u8,
    pub is_feedback: bool,
}

pub struct Algorithm {
    pub number: u8,
    pub name: String,
}

#[derive(Debug)]
pub struct ValidationError {
    pub algorithm: Option<u8>,
    pub error_type: ValidationErrorType,
    pub message: String,
}

#[derive(Debug)]
pub enum ValidationErrorType {
    InvalidOperator,
    InvalidConnection,
    CircularDependency,
    UnreachableCarrier,
    OrphanOperator,
    ProcessingOrder,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
    pub is_valid: bool,
}

static ALGORITHMS: OnceLock<Vec<AlgorithmDef>> = OnceLock::new();

fn load_algorithms() -> &'static Vec<AlgorithmDef> {
    ALGORITHMS.get_or_init(|| {
        let json_content = match fs::read_to_string("algorithms.json") {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read algorithms.json: {}. Using fallback.", e);
                return vec![create_fallback_algorithm()];
            }
        };

        let algorithms = match serde_json::from_str::<Vec<AlgorithmDef>>(&json_content) {
            Ok(algos) => algos,
            Err(e) => {
                error!("Failed to parse algorithms.json: {}. Using fallback.", e);
                return vec![create_fallback_algorithm()];
            }
        };

        // Validate all algorithms
        let validation_report = validate_all_algorithms(&algorithms);
        log_validation_report(&validation_report);

        if !validation_report.is_valid {
            error!("Critical validation errors found. Using fallback algorithms.");
            return vec![create_fallback_algorithm()];
        }

        info!("Loaded {} algorithms successfully", algorithms.len());
        algorithms
    })
}

fn create_fallback_algorithm() -> AlgorithmDef {
    AlgorithmDef {
        algorithm: 32,
        name: "6 Additive (Fallback)".to_string(),
        carriers: vec![1, 2, 3, 4, 5, 6],
        connections: vec![],
    }
}

pub fn find_algorithm(algorithm: u8) -> Option<&'static AlgorithmDef> {
    load_algorithms().iter().find(|a| a.algorithm == algorithm)
}

fn process_operators_iterative(
    ops: &mut [Operator; 6],
    algorithm_def: &AlgorithmDef,
) -> [f32; 6] {
    let mut outputs = [0.0_f32; 6];
    let mut processed = [false; 6];
    
    // Build dependency graph once
    let mut dependencies: [Vec<usize>; 6] = Default::default();
    let mut self_feedback = [false; 6];
    
    for conn in &algorithm_def.connections {
        let from_idx = (conn.from - 1) as usize;
        let to_idx = (conn.to - 1) as usize;
        
        if from_idx < 6 && to_idx < 6 {
            if from_idx == to_idx {
                self_feedback[from_idx] = true; // Self-feedback handled by operator
            } else {
                dependencies[to_idx].push(from_idx);
            }
        }
    }
    
    // CRITICAL: Iterative processing with cycle detection to prevent infinite loops
    let mut max_iterations = 12; // Prevent infinite loops (2 passes per operator max)
    let mut progress_made = true;
    
    while progress_made && max_iterations > 0 {
        progress_made = false;
        max_iterations -= 1;
        
        for op_idx in 0..6 {
            if processed[op_idx] {
                continue;
            }
            
            // Check if all dependencies are processed
            let deps_ready = dependencies[op_idx].iter()
                .all(|&dep_idx| processed[dep_idx]);
                
            if deps_ready {
                // Calculate modulation from dependencies
                let modulation = dependencies[op_idx].iter()
                    .map(|&dep_idx| outputs[dep_idx])
                    .sum::<f32>();
                
                // Process this operator
                outputs[op_idx] = ops[op_idx].process(modulation);
                processed[op_idx] = true;
                progress_made = true;
            }
        }
    }
    
    // SAFETY: Process any remaining unprocessed operators with zero modulation
    for op_idx in 0..6 {
        if !processed[op_idx] {
            outputs[op_idx] = ops[op_idx].process(0.0);
        }
    }
    
    outputs
}

impl Algorithm {
    pub fn process_algorithm(
        algorithm: u8,
        ops: &mut [Operator; 6],
        _base_frequency: f32,
        _velocity: f32,
    ) -> f32 {
        let algorithm_def = match find_algorithm(algorithm) {
            Some(def) => def,
            None => {
                // Fallback: all carriers, no modulation
                return ops.iter_mut().map(|op| op.process(0.0)).sum::<f32>() * 0.16;
            }
        };

        // FIXED: Use iterative processing instead of recursive
        let outputs = process_operators_iterative(ops, algorithm_def);

        // Calculate output from all carriers
        let mut total_output = 0.0;
        let carrier_count = algorithm_def.carriers.len() as f32;

        for &carrier in &algorithm_def.carriers {
            let carrier_idx = (carrier - 1) as usize;
            if carrier_idx < 6 {
                total_output += outputs[carrier_idx];
            }
        }

        // Scale output appropriately for multiple carriers
        if carrier_count > 1.0 {
            total_output / carrier_count.sqrt()
        } else {
            total_output
        }
    }

    pub fn get_all_algorithms() -> Vec<Algorithm> {
        load_algorithms()
            .iter()
            .map(|def| Algorithm {
                number: def.algorithm,
                name: def.name.clone(),
            })
            .collect()
    }

    pub fn parse_algorithm_graph(algorithm: u8) -> AlgorithmGraph {
        let algorithm_def = match find_algorithm(algorithm) {
            Some(def) => def,
            None => {
                // Default fallback
                return AlgorithmGraph {
                    operators: (1..=6)
                        .map(|id| OperatorNode {
                            id,
                            position: (0.0, 0.0),
                            is_carrier: true,
                        })
                        .collect(),
                    connections: vec![],
                };
            }
        };

        // Create operators based on algorithm definition
        let mut operators = vec![];
        for op_id in 1..=6 {
            operators.push(OperatorNode {
                id: op_id,
                position: (0.0, 0.0),
                is_carrier: algorithm_def.carriers.contains(&op_id),
            });
        }

        // Create connections and detect feedback patterns
        let mut connections = vec![];
        for conn_def in &algorithm_def.connections {
            let is_feedback = is_feedback_connection(conn_def, &algorithm_def.connections);
            connections.push(Connection {
                from: conn_def.from,
                to: conn_def.to,
                is_feedback,
            });
        }

        AlgorithmGraph {
            operators,
            connections,
        }
    }

    pub fn calculate_layout(mut graph: AlgorithmGraph, canvas_size: (f32, f32)) -> AlgorithmGraph {
        let (width, height) = canvas_size;
        let row_spacing = 35.0; // Spacing between rows
        let column_spacing = 45.0; // Spacing between columns
        let op_size = 20.0; // Operator circle size

        // Build new grid-based layout
        let grid_layout = Self::build_grid_layout(&graph);
        debug!("Grid Layout - Dimensions: {:?}", grid_layout.dimensions);
        debug!(
            "Grid Layout - Positions: {:?}",
            grid_layout.operator_positions
        );

        // Calculate dimensions for centering
        let (grid_rows, grid_cols) = grid_layout.dimensions;

        let total_width = if grid_cols > 1 {
            (grid_cols - 1) as f32 * column_spacing + op_size
        } else {
            op_size
        };
        let total_height = if grid_rows > 1 {
            (grid_rows - 1) as f32 * row_spacing + op_size
        } else {
            op_size
        };

        // Center the entire grid within the canvas
        let x_offset = (width - total_width) / 2.0 + op_size / 2.0;
        let y_offset = (height - total_height) / 2.0 + op_size / 2.0;

        // Position operators based on grid layout
        for op in &mut graph.operators {
            if let Some(&grid_pos) = grid_layout.operator_positions.get(&op.id) {
                // Convert grid position to canvas coordinates
                // Note: row 0 (carriers) should be at the bottom visually
                let x = x_offset + grid_pos.col as f32 * column_spacing;
                let y = y_offset + (grid_rows - 1 - grid_pos.row) as f32 * row_spacing;

                op.position = (x, y);
                debug!(
                    "Operator {} positioned at grid ({},{}) -> canvas ({:.1},{:.1})",
                    op.id, grid_pos.row, grid_pos.col, x, y
                );
            } else {
                // Fallback position if not found in grid
                warn!(
                    "Operator {} not found in grid layout, using fallback position",
                    op.id
                );
                op.position = (width / 2.0, height / 2.0);
            }
        }

        graph
    }

    // ============================================================================
    // NEW GRID-BASED LAYOUT SYSTEM
    // ============================================================================

    fn build_grid_layout(graph: &AlgorithmGraph) -> GridLayout {
        let mut grid: HashMap<GridPosition, u8> = HashMap::new();
        let mut operator_positions: HashMap<u8, GridPosition> = HashMap::new();

        // Separate carriers and modulators
        let carriers: Vec<u8> = graph
            .operators
            .iter()
            .filter(|op| op.is_carrier)
            .map(|op| op.id)
            .collect();

        let modulators: Vec<u8> = graph
            .operators
            .iter()
            .filter(|op| !op.is_carrier)
            .map(|op| op.id)
            .collect();

        // STEP 1: Place carriers in row 0, evenly spaced
        for (index, &carrier) in carriers.iter().enumerate() {
            let pos = GridPosition { row: 0, col: index };
            grid.insert(pos, carrier);
            operator_positions.insert(carrier, pos);
        }

        // STEP 2: Calculate layers for modulators using simple BFS
        let modulator_layers = Self::calculate_simple_layers(graph, &carriers);
        debug!("Modulator layers: {:?}", modulator_layers);

        // STEP 3: Place modulators by layer with vertical alignment for connected operators
        let max_layer = modulator_layers.values().max().copied().unwrap_or(0);

        for layer in 1..=max_layer {
            // Get modulators in this layer, sorted by ID for consistency
            let mut layer_modulators: Vec<u8> = modulators
                .iter()
                .filter(|&&mod_id| modulator_layers.get(&mod_id) == Some(&layer))
                .copied()
                .collect();
            layer_modulators.sort();

            for modulator in layer_modulators {
                let best_pos = Self::find_vertical_aligned_position(
                    modulator,
                    layer,
                    &graph.connections,
                    &operator_positions,
                    &grid,
                );

                grid.insert(best_pos, modulator);
                operator_positions.insert(modulator, best_pos);
            }
        }

        // Calculate final dimensions
        let max_row = max_layer;
        let max_col = operator_positions
            .values()
            .map(|pos| pos.col)
            .max()
            .unwrap_or(0);

        GridLayout {
            grid,
            operator_positions,
            dimensions: (max_row + 1, max_col + 1),
        }
    }

    #[allow(dead_code)]
    fn get_modulator_chain_size(graph: &AlgorithmGraph, modulator: u8) -> usize {
        // Count how many operators this modulator affects (directly and indirectly)
        let mut visited = HashSet::new();
        Self::count_affected_operators(graph, modulator, &mut visited)
    }

    #[allow(dead_code)]
    fn count_affected_operators(
        graph: &AlgorithmGraph,
        op: u8,
        visited: &mut HashSet<u8>,
    ) -> usize {
        if visited.contains(&op) {
            return 0;
        }
        visited.insert(op);

        let mut count = 1; // Count this operator

        // Count all operators this one affects
        for conn in &graph.connections {
            if conn.from == op && !conn.is_feedback {
                count += Self::count_affected_operators(graph, conn.to, visited);
            }
        }

        visited.remove(&op);
        count
    }

    fn calculate_simple_layers(graph: &AlgorithmGraph, carriers: &[u8]) -> HashMap<u8, usize> {
        let mut layers = HashMap::new();
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        // Start BFS from all carriers (layer 0)
        for &carrier in carriers {
            queue.push_back((carrier, 0));
            visited.insert(carrier);
            layers.insert(carrier, 0);
        }

        // BFS to find shortest path from carriers to modulators
        while let Some((current_op, distance)) = queue.pop_front() {
            // Look for incoming connections (modulators that feed into current_op)
            for conn in &graph.connections {
                if conn.to == current_op && !conn.is_feedback && !visited.contains(&conn.from) {
                    visited.insert(conn.from);
                    layers.insert(conn.from, distance + 1);
                    queue.push_back((conn.from, distance + 1));
                }
            }
        }

        layers
    }

    fn find_vertical_aligned_position(
        modulator: u8,
        layer: usize,
        connections: &[Connection],
        operator_positions: &HashMap<u8, GridPosition>,
        grid: &HashMap<GridPosition, u8>,
    ) -> GridPosition {
        // Find targets that this modulator connects to
        let targets: Vec<u8> = connections
            .iter()
            .filter(|conn| conn.from == modulator && !conn.is_feedback)
            .map(|conn| conn.to)
            .collect();

        // Try to align vertically with targets (prefer same column as target)
        for &target in &targets {
            if let Some(&target_pos) = operator_positions.get(&target) {
                let preferred_pos = GridPosition {
                    row: layer,
                    col: target_pos.col,
                };

                // If position is free, use it for vertical alignment
                if !grid.contains_key(&preferred_pos) {
                    debug!(
                        "Placing modulator {} vertically aligned with target {} at col {}",
                        modulator, target, target_pos.col
                    );
                    return preferred_pos;
                }
            }
        }

        // If no vertical alignment possible, find next available position in layer
        let mut col = 0;
        loop {
            let pos = GridPosition { row: layer, col };
            if !grid.contains_key(&pos) {
                return pos;
            }
            col += 1;
        }
    }
}

// ============================================================================
// FEEDBACK DETECTION HELPERS
// ============================================================================

/// Determines if a connection represents feedback based on DX7 patterns
fn is_feedback_connection(conn: &ConnectionDef, all_connections: &[ConnectionDef]) -> bool {
    // Self-loops are always feedback
    if conn.from == conn.to {
        return true;
    }

    // Check for backward connections that create cycles
    // A connection is feedback if it would create a cycle when added to the forward-only graph
    would_create_cycle(conn, all_connections)
}

/// Checks if adding this connection would create a cycle in the forward-only graph
fn would_create_cycle(new_conn: &ConnectionDef, all_connections: &[ConnectionDef]) -> bool {
    use std::collections::HashMap;

    // Build forward-only graph (excluding potential feedback)
    let mut graph = HashMap::new();
    for conn in all_connections {
        if conn != new_conn && !is_obvious_feedback(conn) {
            graph
                .entry(conn.from)
                .or_insert_with(Vec::new)
                .push(conn.to);
        }
    }

    // Check if adding new_conn would create a cycle
    // This happens if there's already a path from new_conn.to to new_conn.from
    has_path_between(&graph, new_conn.to, new_conn.from)
}

/// Checks for obvious feedback patterns (self-loops)
fn is_obvious_feedback(conn: &ConnectionDef) -> bool {
    conn.from == conn.to
}

/// Checks if there's a path from 'from' to 'to' in the graph
fn has_path_between(graph: &HashMap<u8, Vec<u8>>, from: u8, to: u8) -> bool {
    use std::collections::HashSet;

    let mut visited = HashSet::new();
    dfs_path_exists(graph, from, to, &mut visited)
}

/// DFS to check if path exists
fn dfs_path_exists(
    graph: &HashMap<u8, Vec<u8>>,
    current: u8,
    target: u8,
    visited: &mut HashSet<u8>,
) -> bool {
    if current == target {
        return true;
    }

    if visited.contains(&current) {
        return false;
    }

    visited.insert(current);

    if let Some(neighbors) = graph.get(&current) {
        for &neighbor in neighbors {
            if dfs_path_exists(graph, neighbor, target, visited) {
                return true;
            }
        }
    }

    false
}

// ============================================================================
// VALIDATION SYSTEM
// ============================================================================

fn validate_all_algorithms(algorithms: &[AlgorithmDef]) -> ValidationReport {
    let mut report = ValidationReport {
        errors: Vec::new(),
        warnings: Vec::new(),
        is_valid: true,
    };

    for algo in algorithms {
        let mut algo_report = validate_algorithm(algo);
        report.errors.append(&mut algo_report.errors);
        report.warnings.append(&mut algo_report.warnings);
        if !algo_report.is_valid {
            report.is_valid = false;
        }
    }

    report
}

fn validate_algorithm(algo: &AlgorithmDef) -> ValidationReport {
    let mut report = ValidationReport {
        errors: Vec::new(),
        warnings: Vec::new(),
        is_valid: true,
    };

    // 1. Validate operator ranges
    validate_operator_ranges(algo, &mut report);

    // 2. Validate carriers
    validate_carriers(algo, &mut report);

    // 3. Validate connections
    validate_connections(algo, &mut report);

    // 4. Validate feedback consistency
    validate_feedback_consistency(algo, &mut report);

    // 5. Check for circular dependencies
    validate_circular_dependencies(algo, &mut report);

    // 6. Check carrier reachability
    validate_carrier_reachability(algo, &mut report);

    // 7. Detect orphan operators
    validate_orphan_operators(algo, &mut report);

    // 8. Check processing order issues
    validate_processing_order(algo, &mut report);

    // 9. Validate edge cases for layout system
    validate_layout_edge_cases(algo, &mut report);

    report
}

fn validate_operator_ranges(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Check carriers
    for &carrier in &algo.carriers {
        if !(1..=6).contains(&carrier) {
            add_error(
                report,
                algo.algorithm,
                ValidationErrorType::InvalidOperator,
                format!("Invalid carrier operator {}: must be 1-6", carrier),
            );
        }
    }

    // Check connections
    for conn in &algo.connections {
        if !(1..=6).contains(&conn.from) {
            add_error(
                report,
                algo.algorithm,
                ValidationErrorType::InvalidOperator,
                format!(
                    "Invalid connection from operator {}: must be 1-6",
                    conn.from
                ),
            );
        }
        if !(1..=6).contains(&conn.to) {
            add_error(
                report,
                algo.algorithm,
                ValidationErrorType::InvalidOperator,
                format!("Invalid connection to operator {}: must be 1-6", conn.to),
            );
        }
    }

    // Note: Feedback is now handled via self-loop connections (from: X, to: X)
}

fn validate_carriers(algo: &AlgorithmDef, report: &mut ValidationReport) {
    if algo.carriers.is_empty() {
        add_error(
            report,
            algo.algorithm,
            ValidationErrorType::InvalidOperator,
            "Algorithm must have at least one carrier".to_string(),
        );
    }

    // Check for duplicate carriers
    let mut seen = HashSet::new();
    for &carrier in &algo.carriers {
        if !seen.insert(carrier) {
            add_warning(
                report,
                algo.algorithm,
                ValidationErrorType::InvalidOperator,
                format!("Duplicate carrier operator {}", carrier),
            );
        }
    }
}

fn validate_connections(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Self-connections (from: X, to: X) are now automatically treated as feedback
    // No additional validation needed as they are the feedback mechanism

    // Check for duplicate connections
    let mut seen = HashSet::new();
    for conn in &algo.connections {
        let key = (conn.from, conn.to);
        if !seen.insert(key) {
            add_warning(
                report,
                algo.algorithm,
                ValidationErrorType::InvalidConnection,
                format!("Duplicate connection {}->{}", conn.from, conn.to),
            );
        }
    }
}

fn validate_feedback_consistency(algo: &AlgorithmDef, _report: &mut ValidationReport) {
    // Check for self-loop connections (feedback)
    for conn in &algo.connections {
        if conn.from == conn.to {
            // Self-loops are valid feedback connections
            // Additional validation could be added here if needed
        }
    }
}

fn validate_circular_dependencies(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Build adjacency graph (excluding feedback connections)
    let mut graph = HashMap::new();
    for conn in &algo.connections {
        // Exclude connections identified as feedback from cycle detection
        if !is_feedback_connection(conn, &algo.connections) {
            graph
                .entry(conn.from)
                .or_insert_with(Vec::new)
                .push(conn.to);
        }
    }

    // DFS to detect cycles
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for op in 1..=6 {
        if !visited.contains(&op) && has_cycle(&graph, op, &mut visited, &mut rec_stack) {
            add_error(
                report,
                algo.algorithm,
                ValidationErrorType::CircularDependency,
                format!("Circular dependency detected involving operator {}", op),
            );
        }
    }
}

fn has_cycle(
    graph: &HashMap<u8, Vec<u8>>,
    node: u8,
    visited: &mut HashSet<u8>,
    rec_stack: &mut HashSet<u8>,
) -> bool {
    visited.insert(node);
    rec_stack.insert(node);

    if let Some(neighbors) = graph.get(&node) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if has_cycle(graph, neighbor, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&neighbor) {
                return true;
            }
        }
    }

    rec_stack.remove(&node);
    false
}

fn validate_carrier_reachability(algo: &AlgorithmDef, report: &mut ValidationReport) {
    for &carrier in &algo.carriers {
        if !is_carrier_reachable(algo, carrier) {
            add_warning(
                report,
                algo.algorithm,
                ValidationErrorType::UnreachableCarrier,
                format!(
                    "Carrier {} may not be reachable through modulation chain",
                    carrier
                ),
            );
        }
    }
}

fn is_carrier_reachable(algo: &AlgorithmDef, carrier: u8) -> bool {
    // A carrier is reachable if it has no incoming connections (standalone)
    // or if all its modulators are reachable
    let incoming: Vec<u8> = algo
        .connections
        .iter()
        .filter(|c| c.to == carrier && c.from != carrier)
        .map(|c| c.from)
        .collect();

    if incoming.is_empty() {
        return true; // Standalone carrier
    }

    // Check if we can reach all modulators (simplified check)
    // In a full implementation, we'd do a complete reachability analysis
    for modulator in incoming {
        if !(1..=6).contains(&modulator) {
            return false;
        }
    }

    true
}

fn validate_orphan_operators(algo: &AlgorithmDef, report: &mut ValidationReport) {
    for op in 1..=6 {
        let is_carrier = algo.carriers.contains(&op);
        let has_outgoing = algo.connections.iter().any(|c| c.from == op);
        let has_incoming = algo.connections.iter().any(|c| c.to == op);

        if !is_carrier && !has_outgoing && !has_incoming {
            add_warning(
                report,
                algo.algorithm,
                ValidationErrorType::OrphanOperator,
                format!("Operator {} is orphaned (no connections)", op),
            );
        }
    }
}

fn validate_processing_order(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Check for potential processing order issues with current recursive implementation
    let mut operators_with_multiple_outputs = Vec::new();

    for op in 1..=6 {
        let output_count = algo
            .connections
            .iter()
            .filter(|c| c.from == op && c.from != c.to)
            .count();
        if output_count > 1 {
            operators_with_multiple_outputs.push(op);
        }
    }

    if !operators_with_multiple_outputs.is_empty() {
        add_warning(
            report,
            algo.algorithm,
            ValidationErrorType::ProcessingOrder,
            format!(
                "Operators with multiple outputs may cause processing issues: {:?}",
                operators_with_multiple_outputs
            ),
        );
    }
}

fn add_error(
    report: &mut ValidationReport,
    algo: u8,
    error_type: ValidationErrorType,
    message: String,
) {
    report.errors.push(ValidationError {
        algorithm: Some(algo),
        error_type,
        message,
    });
    report.is_valid = false;
}

fn add_warning(
    report: &mut ValidationReport,
    algo: u8,
    error_type: ValidationErrorType,
    message: String,
) {
    report.warnings.push(ValidationError {
        algorithm: Some(algo),
        error_type,
        message,
    });
}

fn validate_layout_edge_cases(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Edge Case 1: All operators are carriers (no modulators)
    if algo.carriers.len() == 6 {
        add_warning(
            report,
            algo.algorithm,
            ValidationErrorType::ProcessingOrder,
            "Algorithm has all operators as carriers - no FM synthesis will occur".to_string(),
        );
    }

    // Edge Case 2: Single carrier with many modulators (layout stress test)
    if algo.carriers.len() == 1 {
        let modulator_count = 6 - algo.carriers.len();
        if modulator_count >= 4 {
            add_warning(
                report,
                algo.algorithm,
                ValidationErrorType::ProcessingOrder,
                format!(
                    "Single carrier with {} modulators may cause layout density issues",
                    modulator_count
                ),
            );
        }
    }

    // Edge Case 3: Complex modulator chains (multiple path lengths)
    validate_modulator_chain_complexity(algo, report);

    // Edge Case 4: Algorithms with disconnected components
    validate_connectivity(algo, report);

    // Edge Case 5: Too many feedback loops (layout complexity)
    let feedback_count = algo
        .connections
        .iter()
        .filter(|conn| conn.from == conn.to)
        .count();
    if feedback_count > 3 {
        add_warning(
            report,
            algo.algorithm,
            ValidationErrorType::ProcessingOrder,
            format!(
                "High feedback count ({}) may complicate layout",
                feedback_count
            ),
        );
    }
}

fn validate_modulator_chain_complexity(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Calculate maximum chain depth from any carrier
    let mut max_depth = 0;

    for &carrier in &algo.carriers {
        let depth = calculate_max_chain_depth(algo, carrier);
        max_depth = max_depth.max(depth);
    }

    if max_depth > 4 {
        add_warning(
            report,
            algo.algorithm,
            ValidationErrorType::ProcessingOrder,
            format!(
                "Deep modulator chain (depth {}) may cause layout challenges",
                max_depth
            ),
        );
    }
}

fn calculate_max_chain_depth(algo: &AlgorithmDef, start_op: u8) -> usize {
    let mut max_depth = 0;
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back((start_op, 0));

    while let Some((current_op, depth)) = queue.pop_front() {
        if visited.contains(&current_op) {
            continue;
        }
        visited.insert(current_op);
        max_depth = max_depth.max(depth);

        // Follow incoming connections (backward from carriers toward modulators)
        for conn in &algo.connections {
            if conn.to == current_op
                && !is_feedback_connection(conn, &algo.connections)
                && !visited.contains(&conn.from)
            {
                queue.push_back((conn.from, depth + 1));
            }
        }
    }

    max_depth
}

fn validate_connectivity(algo: &AlgorithmDef, report: &mut ValidationReport) {
    // Find all connected components
    let mut visited = HashSet::new();
    let mut components = Vec::new();

    for op in 1..=6 {
        if !visited.contains(&op) {
            let component = find_connected_component(algo, op, &mut visited);
            components.push(component);
        }
    }

    // Special case: If all operators are carriers, they're connected via output mixing
    if algo.carriers.len() == 6 {
        // All carriers - they're connected through the output mixer, no warning needed
        return;
    }

    // Check for truly disconnected components (exclude single carriers)
    let non_carrier_components: Vec<&Vec<u8>> = components
        .iter()
        .filter(|component| {
            // Keep components that are either:
            // - Size > 1 (multiple operators connected)
            // - Single modulator (not a carrier)
            component.len() > 1 || !algo.carriers.contains(&component[0])
        })
        .collect();

    // Only warn if we have multiple meaningful components
    if non_carrier_components.len() > 1 {
        let component_sizes: Vec<usize> = non_carrier_components.iter().map(|c| c.len()).collect();
        add_warning(
            report,
            algo.algorithm,
            ValidationErrorType::OrphanOperator,
            format!(
                "Algorithm has {} disconnected modulator chains with sizes: {:?}",
                non_carrier_components.len(),
                component_sizes
            ),
        );
    }
}

fn find_connected_component(
    algo: &AlgorithmDef,
    start_op: u8,
    visited: &mut HashSet<u8>,
) -> Vec<u8> {
    let mut component = Vec::new();
    let mut queue = VecDeque::new();

    queue.push_back(start_op);

    while let Some(current_op) = queue.pop_front() {
        if visited.contains(&current_op) {
            continue;
        }

        visited.insert(current_op);
        component.push(current_op);

        // Add all connected operators (both directions)
        for conn in &algo.connections {
            if conn.from == current_op && !visited.contains(&conn.to) {
                queue.push_back(conn.to);
            }
            if conn.to == current_op && !visited.contains(&conn.from) {
                queue.push_back(conn.from);
            }
        }
    }

    component
}

fn log_validation_report(report: &ValidationReport) {
    if report.is_valid {
        info!("Algorithm validation passed");
    } else {
        error!(
            "Algorithm validation failed with {} errors",
            report.errors.len()
        );
    }

    for error in &report.errors {
        error!(
            "[Algo {}] {:?}: {}",
            error.algorithm.unwrap_or(0),
            error.error_type,
            error.message
        );
    }

    for warning in &report.warnings {
        warn!(
            "[Algo {}] {:?}: {}",
            warning.algorithm.unwrap_or(0),
            warning.error_type,
            warning.message
        );
    }
}

// Public function to validate algorithms on demand
